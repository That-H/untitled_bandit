//! Contains code for generating individual floors of the game.

use super::*;
use crate::*;
use entity::*;
use std::collections::HashMap;
use templates::metadata::TempMeta;
use tile_presets::*;

/// Number of rooms on floor 0.
pub const ROOMS: u32 = 9;
/// Number of rooms generated without map_gen::map_gen.
pub const SPECIAL_ROOMS: u32 = 2;
/// Maximum width or height a room can be.
pub const MAX_WIDTH: i32 = 13;
/// Minimum width or height a room can be.
pub const MIN_WIDTH: i32 = 6;

/// True if the map should be generated with bonus ice puzzle rooms.
pub const EXTRA_ICE: bool = if cfg!(debug_assertions) { false } else { false };

fn get_temp<'a>(
    budget: u32,
    rng: &mut rand::rngs::SmallRng,
    elite: bool,
    temp_counts: &HashMap<char, u32>,
    meta: &HashMap<char, TempMeta>,
    templates: &'a [EntityTemplate],
    elites: &'a [EntityTemplate],
) -> Option<(&'a EntityTemplate, u32)> {
    let temps = if elite { elites } else { templates };
    let possible: Vec<_> = temps
        .iter()
        .filter_map(|t| {
            let ch = t.ch.content();
            let TempMeta {
                cost,
                floor_rang: flrs,
                max,
            } = &meta[ch];
            let cost = *cost;
            if cost <= budget
                && flrs.contains(&unsafe { FLOORS_CLEARED })
                && match temp_counts.get(ch) {
                    Some(c) => c < max,
                    _ => true,
                }
            {
                Some((t, cost))
            } else {
                None
            }
        })
        .collect();
    possible.choose(rng).cloned()
}

/// Generate a single floor of an untitled_bandit game.
pub fn gen_floor(
    map: &mut bandit::Map<En>,
    rng: &mut rand::rngs::SmallRng,
    floor_num: u32,
    meta: &HashMap<char, TempMeta>,
    templates: &[EntityTemplate],
    elites: &[EntityTemplate],
) {
    // Create the player if it is the first floor, otherwise get them.
    let pl = if floor_num == 0 {
        templates::get_player()
    } else {
        map.get_ent(unsafe { PLAYER }).unwrap().clone()
    };

    // Reinitialise the map.
    *map = bandit::Map::new(0, 0);

    unsafe {
        PLAYER = Point::ORIGIN;
        LAST_DOOR.write().unwrap().take();
    }
    map.insert_entity(pl, unsafe { PLAYER });

    let ice_prevalence = if EXTRA_ICE { 1.0 } else { 0.15 };

    // Generate the rooms of the map.
    let (mut grid, mut rooms) = map_gen::map_gen(
        ROOMS - SPECIAL_ROOMS + floor_num * 3,
        MAX_WIDTH,
        MIN_WIDTH,
        rng,
    );

    // Rect id of the exit room so that the key room does not generate off of it.
    let exit_id: usize = rooms.len();

    // Generate a new room specifically for the boss.
    let true_door = map_gen::gen_rect_in(&mut rooms, &mut grid, rng, MIN_WIDTH, MAX_WIDTH, &[0]);
    let mut exit_pos = rooms.last().unwrap().top_left();

    // Generate a new room specifically for the key.
    let key_door = map_gen::gen_rect_in(
        &mut rooms,
        &mut grid,
        rng,
        MIN_WIDTH,
        MAX_WIDTH,
        &[0, exit_id],
    );

    if crate::CHEATS {
        LOG_MSGS
            .write()
            .unwrap()
            .push(LogMsg::new(format!("Key door at {key_door}")));
    }

    let rm_len = rooms.len();

    // Create some ice puzzles.
    map_gen::add_ice(
        &mut rooms,
        &mut grid,
        rng,
        ice_prevalence,
        // Do not touch the key room.
        &[rm_len - 1],
    );

    // Find a suitable position for the exit.
    loop {
        if let Some(cl) = grid.get(&exit_pos)
            && let Cell::Inner(_) = cl 
            && exit_pos.get_all_adjacent().into_iter().all(|p| if let Some(cl) = grid.get(&p) && cl.is_door() { false } else { true })
        {
            break;
        } else {
            exit_pos = exit_pos + Point::new(1, -1);
        }
    }

    // Place the exit tile.
    map.insert_tile(
        get_exit(false, floor_num as usize),
        exit_pos
    );

    // Generate enemies.
    for (n, r) in rooms.iter().enumerate().skip(1) {
        let mut budget = (r.wid * r.hgt) as u32 / 3 * unsafe { FLOORS_CLEARED + 1 };
        let mut cells: Vec<Point> = r.inner_cells().collect();
        cells.shuffle(rng);

        // Create a boss in the exit room.
        let elite = n == exit_id;

        // Don't put anyone in my key room!
        if n == rooms.len() - 1 {
            continue;
        }

        let mut temp_counts = HashMap::new();

        if elite {
            budget = 75;
        }

        'enemy_gen: while let Some((temp, cost)) =
            get_temp(budget, rng, elite, &temp_counts, meta, templates, elites)
        {
            budget -= cost;
            let nx = loop {
                // Exit early if there is no where to place the entity.
                let Some(nx) = cells.pop() else {
                    break 'enemy_gen;
                };

                let Some(tl) = grid.get(&nx) else { continue };
                match tl {
                    map_gen::Cell::Ice(_) | map_gen::Cell::Wall(_) => continue,
                    _ => break nx,
                }
            };

            *temp_counts.entry(*temp.ch.content()).or_insert(0) += 1;

            map.insert_entity(En::from_template(temp, false, true), nx);
        }
    }

    let rm = rooms.last().unwrap();
    let key_pos = rm.top_left() + Point::new(rm.wid / 2, -rm.hgt / 2);

    // Put the cells actually into the map.
    for (&pos, cl) in grid.iter() {
        let blocking;
        let mut slippery = false;
        let mut door = false;

        // If there is already a tile there, don't overwrite it.
        if map.get_map(pos).is_some() {
            continue;
        };

        let ch = match cl {
            map_gen::Cell::Wall(_) => {
                blocking = true;
                None
            }
            map_gen::Cell::Inner(_) => {
                blocking = false;
                None
            }
            map_gen::Cell::Ice(_) => {
                blocking = false;
                slippery = true;
                Some(ICE_CHAR.on(ICE_CLR))
            }
            map_gen::Cell::Door(_id1, _id2) => {
                blocking = false;
                door = true;
                Some(DOOR_CHAR.with(get_door_clr()))
            }
        };

        let revealed = rooms[0].contains(pos);
        let t = if pos == key_pos {
            get_key(false, floor_num)
        } else {
            Tile {
                ch,
                blocking,
                empt: false,
                revealed,
                slippery,
                door,
                step_effect: None,
                locked: None,
            }
        };

        map.insert_tile(t, pos);
    }

    let door = map.get_map_mut(true_door).unwrap();
    door.ch = Some('╬'.with(KEY_CLRS[floor_num as usize]));
    door.locked = Some(floor_num);
    door.blocking = true;
}
