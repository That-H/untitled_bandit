//! Contains code for generating individual floors of the game.

use super::*;
use crate::*;
use entity::*;
use std::collections::HashMap;
use templates::metadata::TempMeta;
use tile_presets::*;
use rand::seq::IteratorRandom;

/// Number of rooms on floor 0.
pub const ROOMS: u32 = 9;
/// Number of rooms generated without map_gen::map_gen.
pub const SPECIAL_ROOMS: u32 = 2;
/// Maximum width or height a room can be.
pub const MAX_WIDTH: i32 = 13;
/// Minimum width or height a room can be.
pub const MIN_WIDTH: i32 = 6;
/// Width and height of the rooms on floor 4.
pub const F4_RM_SIZE: i32 = 15;
/// Number of rooms on floor 4.
pub const F4_ROOMS: usize = 25;

/// True if the map should be generated with bonus ice puzzle rooms.
pub const EXTRA_ICE: bool = if cfg!(debug_assertions) { false } else { false };

fn get_temp<'a, R: rand::Rng>(
    budget: u32,
    rng: &mut R,
    temp_counts: &HashMap<char, u32>,
    meta: &HashMap<char, TempMeta>,
    templates: &'a [EntityTemplate],
    floor_num: u32,
) -> Option<(&'a EntityTemplate, u32)> {
    let possible: Vec<_> = templates
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
                && flrs.contains(&floor_num)
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
pub fn gen_floor<R: rand::Rng>(
    map: &mut bandit::Map<En>,
    rng: &mut R,
    floor_num: u32,
    meta: &HashMap<char, TempMeta>,
    templates: &[EntityTemplate],
    elites: &[EntityTemplate],
) {
    // Display a message saying that we entered the floor.
    let flr_text = if floor_num < 5 { floor_num.to_string() } else { String::from("???") };
    LOG_MSGS.write().unwrap().push(LogMsg::new(format!("{} enters floor {flr_text}", templates::PLAYER_CHARACTER)));

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
    let rooms = if floor_num >= 4 { 1 } else { ROOMS - SPECIAL_ROOMS + floor_num * 3 };

    // Generate the rooms of the map.
    let (mut grid, mut rooms) = map_gen::map_gen(
        rooms,
        if floor_num == 4 { F4_RM_SIZE } else { MAX_WIDTH },
        if floor_num == 4 { F4_RM_SIZE } else { MIN_WIDTH },
        rng,
    );

    // Ids of all key rooms so we know not to put enemies in them.
    let mut key_ids = Vec::new();
    let mut true_door = None;
    // Default value so it doesn't appear on floor 5.
    let mut exit_pos: Point = Point::new(100, 100);
    let mut exit_id: usize = 0;

    // "Normal generation" if not on floor 4.
    if floor_num < 4 {
        // Rect id of the exit room so that the key room does not generate off of it.
        exit_id = rooms.len();

        // Generate a new room specifically for the boss.
        true_door = Some(map_gen::gen_rect_in(&mut rooms, &mut grid, rng, MIN_WIDTH, MAX_WIDTH, &[0]));
        exit_pos = rooms.last().unwrap().top_left();

        // Generate a new room(s) specifically for the key(s).
        let mut ill_hosts = vec![0, exit_id];
        let key_tot = if floor_num == KILL_SCREEN as u32 - 1 { 4 } else { 1 };
        for mut i in 0..key_tot {
            key_ids.push(rooms.len());
            ill_hosts.push(rooms.len());
            if key_tot == 1 {
                i = floor_num;
            }
            gen_key_room(rng, map, &mut rooms, &mut grid, i, &ill_hosts);
        }

        // Create some ice puzzles.
        map_gen::add_ice(
            &mut rooms,
            &mut grid,
            rng,
            ice_prevalence,
            // Do not touch the key room(s).
            &ill_hosts
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
    } else if floor_num == 4 {
        // Special maze generation.
        let mut conns: Vec<Vec<Point>> = vec![Vec::new()];

        for id in 1..=F4_ROOMS {
            let (host, door_pos, dir) = loop {
                let Some((host, _)) = conns
                    .iter()
                    .enumerate()
                    .filter(|(_id, v)| v.len() < 4)
                    .choose(rng)
                else { unreachable!("couldn't find a suitable host") };
                let host_rm = &rooms[host];

                let rm_centre = host_rm.top_left() + Point::new(host_rm.wid / 2, host_rm.hgt / -2);
                let Some((door_pos, dir)) = Point::ORIGIN
                    .get_all_adjacent()
                    .into_iter()
                    .filter_map(|p| {
                        if conns[host].contains(&p) {
                            return None;
                        }
                        let edge_mid = (p * (F4_RM_SIZE / 2) as i32) + rm_centre;

                        if let Some(Cell::Wall(ids)) = grid.get(&edge_mid) && ids.len() <= 1 {
                            Some((edge_mid, p))
                        } else {
                            conns[host].push(p);
                            None
                        }
                    })
                    .choose(rng)
                else { 
                    continue;
                };
                break (host, door_pos, dir);
            };

            conns[host].push(dir);
            if conns.len() == id {
                conns.push(Vec::new());
            }
            conns[id].push(-dir);

            let mut new_rm = Rect::new(door_pos.x, door_pos.y, 1, 1);
            new_rm.expand(dir * (F4_RM_SIZE - 1));
            new_rm.expand(dir.rotate_90_cw() * (F4_RM_SIZE / 2));
            new_rm.expand(dir.rotate_90_acw() * (F4_RM_SIZE / 2));
            map_gen::insert_rect(&mut rooms, &mut grid, new_rm, host, door_pos);
        }

        exit_id = rng.random_range(1..rooms.len());
        // Reversed so rooms generated later are more likely to be the key rooms.
        let mut times_keyed = 0;
        for n in (1..rooms.len()).rev() {
            if n == exit_id { 
                continue;
            }
            if conns[n].len() == 1 {
                if times_keyed < 4 {
                    let rm = &rooms[n];
                    let rm_centre = rm.top_left() + Point::new(rm.wid / 2, rm.hgt / -2);
                    map.insert_tile(get_key(false, times_keyed), rm_centre);
                    times_keyed += 1;
                    key_ids.push(n);
                }
            } else if rng.random_bool(0.15) {
                map_gen::ice_rect(&mut rooms, &mut grid, rng, n, 0.1, 5);
            }
        }

        // Create exit with locked doors to it.
        let exit_rm = rooms[exit_id];
        exit_pos = exit_rm.top_left() + Point::new(exit_rm.wid / 2, exit_rm.hgt / -2);
        let wll = Tile {
            empt: false,
            blocking: true,
            ch: Some('#'.with(WALL_CLRS[5])),
            revealed: false,
            door: false,
            locked: None,
            slippery: false,
            step_effect: None,
        };
        for p in exit_pos.get_all_adjacent_diagonal() {
            map.insert_tile(wll.clone(), p);
        }
        let door_dir = Point::ORIGIN.get_all_adjacent()[rng.random_range(0..4)];
        let mut cur_pos = exit_pos;
        for door_clr in (0..4).rev() {
            cur_pos = cur_pos + door_dir; 
            map.insert_tile(get_locked_door(false, door_clr), cur_pos);
            map.insert_tile(wll.clone(), cur_pos + door_dir.rotate_90_cw());
            map.insert_tile(wll.clone(), cur_pos + door_dir.rotate_90_acw());
        }
    }

    // Generate enemies.
    for (n, r) in rooms.iter().enumerate() {
        if n == 0 && floor_num != 5 {
            continue;
        }
        // Don't put anyone in my key room(s)!
        if key_ids.contains(&n) {
            continue;
        }

        let mut area = 0;
        let mut cells = Vec::new();
        if floor_num == 5 {
            cells.push(Point::new(0, 4));
        } else {
            for pos in r.inner_cells() {
                // Make sure the cell is a floor tile that isn't near a door.
                if let Some(Cell::Inner(_)) = grid.get(&pos) 
                    && Rect::new(-2, 2, 5, 5)
                        .cells()
                        .all(|p| {
                            match grid.get(&(p + pos)) {
                                Some(t) => !t.is_door(),
                                None => true,
                            }
                        })
                {
                    area += 1;
                    cells.push(pos);
                }
            }
        }

        let mut budget = area as u32 + floor_num * 30;

        // Create a boss in the exit room.
        let elite = n == exit_id;
        let mut over_ride = floor_num == 5;

        let f_num: u32;

        if floor_num == 4 && rng.random_bool(0.3) && !elite {
            over_ride = true;
            f_num = rng.random_range(1..=3);
        } else {
            f_num = floor_num;
        }

        let templates = if elite || over_ride { 
            budget = if over_ride { 125 } else { 75 };
            elites
        } else {
            templates 
        };

        populate(rng, budget, map, templates, meta, &cells, f_num);
    }

    // Place the exit tile.
    map.insert_tile(
        get_exit(false, floor_num as usize),
        exit_pos
    );

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
                Some(ICE_CHAR.with(ICE_CLR))
            }
            map_gen::Cell::Door(_id1, _id2) => {
                blocking = false;
                door = true;
                Some(DOOR_CHAR.with(get_door_clr()))
            }
        };

        let revealed = rooms[0].contains(pos);
        let t = Tile {
            ch,
            blocking,
            empt: false,
            revealed,
            slippery,
            door,
            step_effect: None,
            locked: None,
        };

        map.insert_tile(t, pos);
    }

    if let Some(true_door) = true_door {
        let door = map.get_map_mut(true_door).unwrap();
        door.ch = Some(LOCKED_DOOR.with(KEY_CLRS[floor_num as usize % 4]));
        door.locked = Some(floor_num);
        door.blocking = true;
    }
}

/// Puts some enemies into the room.
fn populate<R: Rng>(
    rng: &mut R,
    budget: u32,
    map: &mut bandit::Map<En>,
    templates: &[EntityTemplate],
    meta: &HashMap<char, TempMeta>,
    valid: &[Point],
    floor_num: u32,
) {
    let mut budget = budget;
    let mut cells = Vec::from(valid);
    cells.shuffle(rng);

    let mut temp_counts = HashMap::new();

    'enemy_gen: while let Some((temp, cost)) =
        get_temp(budget, rng, &temp_counts, meta, templates, floor_num)
    {
        budget -= cost;
        // Exit early if there is no where to place the entity.
        let Some(nx) = cells.pop() else {
            break 'enemy_gen;
        };

        *temp_counts.entry(*temp.ch.content()).or_insert(0) += 1;
        let mut en = En::from_template(temp, false, true);
        if floor_num == 5 {
            en.dormant = false;
            en.special = Special::FinalBoss;
            unsafe { ENEMIES_REMAINING += 1 }
        }
        en.acted = true;

        map.insert_entity(en, nx);
    }
}

/// Generate a key room in the map.
fn gen_key_room<R: Rng>(
    rng: &mut R,
    map: &mut bandit::Map<En>,
    rects: &mut Vec<Rect>,
    occupied: &mut HashMap<Point, Cell>,
    key_clr: u32,
    ill_hosts: &[usize],
) {
    let key_door = map_gen::gen_rect_in(
        rects,
        occupied,
        rng,
        MIN_WIDTH,
        MAX_WIDTH,
        &ill_hosts,
    );

    if crate::CHEATS {
        LOG_MSGS
            .write()
            .unwrap()
            .push(LogMsg::new(format!("Key door at {key_door}")));
    }

    let rm = rects.last().unwrap();
    let key_pos = Point::new((rm.left + rm.wid / 2) as i32, (rm.top - rm.hgt / 2) as i32);
    map.insert_tile(get_key(false, key_clr), key_pos);
}
