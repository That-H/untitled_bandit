#![allow(unused_must_use)]

use attacks::*;
use bn::windowed;
use crossterm::style::{self, Stylize};
use crossterm::{cursor, event, execute, queue, terminal};
use entity::*;
use io::Write;
use rand::prelude::{IndexedRandom, SliceRandom};
use rand::{Rng, SeedableRng};
use std::{io, thread, collections::HashMap};
use untitled_bandit::*;

const ROOMS: u32 = 50;
const MAX_WIDTH: i32 = 13;
const MIN_WIDTH: i32 = 6;
const MAP_WIDTH: usize = 300;
const MAP_HEIGHT: usize = 300;
const TERMINAL_WID: u16 = 120;
const TERMINAL_HGT: u16 = 30;
const WINDOW_WIDTH: u16 = 40;
const WINDOW_HEIGHT: u16 = 20;
const ARROWS: [char; 4] = ['↓', '←', '↑', '→'];

// All constants below describe the index of the window container that
// the corresponding window is located at, or things about the window.
const GAME: usize = 0;
const STATS: usize = 1;
const STATS_POS: Point = Point::new(10, 5);
const STATS_WID: usize = 15;
const ATKS: usize = 2;
const ATKS_POS: Point = Point::new(10, 12);
const ATKS_WID: usize = 9;

fn create_conveyor(disp: Point, revealed: bool) -> Tile {
    let step_effect: Option<Box<dyn Fn(Point, &bn::Map<En>) -> Vec<bn::Cmd<En>>>> =
        Some(Box::new(move |pos: Point, _map: &bn::Map<En>| {
            vec![
                bn::Cmd::new_on(pos).modify_entity(Box::new(move |e: &mut En| {
                    e.vel = Some(disp);
                })),
            ]
        }));
    let dir = disp.dir();
    Tile {
        ch: Some(ARROWS[dir].green()),
        blocking: false,
        empt: false,
        revealed,
        slippery: false,
        door: None,
        step_effect,
    }
}

fn main() {
    // Rng used for map generation. Has to be separate to ensure determinism
    // with the map and its contents.
    let mut floor_rng = rand::rngs::StdRng::from_os_rng();

    // Generate a map.
    let (grid, rooms) = map_gen::map_gen(ROOMS, MAX_WIDTH, MIN_WIDTH, &mut floor_rng);

    let mut map: bn::Map<En> = bn::Map::new(MAP_WIDTH, MAP_HEIGHT);

    terminal::enable_raw_mode();
    execute!(io::stdout(), terminal::Clear(terminal::ClearType::All));

    for y in -(((MAP_HEIGHT / 2) + MAP_OFFSET) as i32)..(MAP_HEIGHT / 2 + MAP_OFFSET) as i32 {
        for x in -(((MAP_WIDTH / 2) + MAP_OFFSET) as i32)..(MAP_WIDTH / 2 + MAP_OFFSET) as i32 {
            let pos = Point::new(x, y);
            let mut blocking = false;
            let mut door = None;
            let ch = match grid.get(&pos) {
                Some(cl) => match cl {
                    map_gen::Cell::Wall(_) => {
                        blocking = true;
                        None
                    }
                    map_gen::Cell::Inner(_) => {
                        blocking = false;
                        None
                    }
                    map_gen::Cell::Door(id1, id2) => {
                        blocking = false;
                        door = Some((rooms[*id1].clone(), rooms[*id2].clone()));
                        Some('/'.yellow())
                    }
                },
                None => None,
            };
            let revealed = rooms[0].contains(Point::new(x, y));
            let t = if floor_rng.random_bool(0.0) && !blocking && door.is_none() {
                create_conveyor(
                    *Point::ORIGIN
                        .get_all_adjacent()
                        .choose(&mut floor_rng)
                        .unwrap(),
                    revealed,
                )
            } else {
                Tile {
                    ch,
                    blocking,
                    empt: false,
                    revealed,
                    slippery: false,
                    door,
                    step_effect: None,
                }
            };

            map.insert_tile(t, pos);
        }
    }

    // Generate knight moves without typing them all out.
    let mut p1 = Point::new(2, 1);
    let mut p2 = Point::new(2, -1);

    let mut knight = Vec::new();

    for _ in 0..4 {
        knight.push(p1);
        knight.push(p2);
        p1.rotate_90_cw_ip();
        p2.rotate_90_cw_ip();
    }

    // Manhattan movement.
    let manhattan = Point::ORIGIN.get_all_adjacent();

    // Manhattan movement with diagonal.
    let total = Point::ORIGIN.get_all_adjacent_diagonal();

    // Default attack pattern.
    let default_atks = AtkPat::from_atks(MeleeAtk::bulk_new(
        DmgInst::dmg(1, 1.0),
        style::Color::Red,
        7,
        Vfx::new_opaque('?'.stylize(), 7),
        4,
    ));

    // Default attack pattern with diagonals included.
    let diagonal_atks = AtkPat::from_atks(MeleeAtk::bulk_new(
        DmgInst::dmg(1, 1.0),
        style::Color::Red,
        7,
        Vfx::new_opaque('?'.stylize(), 7),
        8,
    ));

    // Long default attack.
    let mut spear = default_atks.clone();

    for (_d, atks) in spear.melee_atks.iter_mut() {
        for atk in atks.iter_mut() {
            let pos = atk.place[0];
            atk.fx
                .push((pos * 2, Vfx::new_opaque(DIR_CHARS[pos.dir()].red(), 7)));
            for p in atk.place.iter_mut() {
                *p = *p * 2;
            }
        }
    }

    let templates = [
        EntityTemplate {
            max_hp: 3,
            delay: 2,
            movement: manhattan.clone(),
            ch: 'e'.stylize(),
            atks: default_atks.clone(),
        },
        EntityTemplate {
            max_hp: 2,
            delay: 1,
            movement: manhattan.clone(),
            ch: 'l'.stylize(),
            atks: spear.clone(),
        },
        EntityTemplate {
            max_hp: 2,
            delay: 2,
            movement: knight.clone(),
            ch: 'k'.stylize(),
            atks: diagonal_atks.clone(),
        },
    ];

    let costs = [10, 25, 40];

    // Create the player.
    let pl = En::new(
        9,
        true,
        0,
        '@'.green(),
        Special::Not,
        total.clone(),
        default_atks.clone(),
        false,
    );

    map.insert_entity(pl, unsafe { PLAYER });

    let get_temp = |budget: u32, rng: &mut rand::rngs::StdRng| -> Option<(&EntityTemplate, u32)> {
        let max_idx = match costs.binary_search(&budget) {
            Ok(idx) => idx,
            Err(idx) => {
                if idx == 0 {
                    return None;
                } else {
                    idx - 1
                }
            }
        };
        let idx = rng.random_range(0..=max_idx);

        Some((&templates[idx], costs[idx]))
    };

    // Generate enemies.
    for r in rooms.iter().skip(1) {
        let mut budget = (r.wid * r.hgt) as u32 / 2;
        let mut cells: Vec<Point> = r.inner_cells().collect();
        cells.shuffle(&mut floor_rng);

        'enemy_gen: while let Some((temp, cost)) = get_temp(budget, &mut floor_rng) {
            budget -= cost;
            let nx = loop {
                // Exit early if there is no where to place the entity.
                let Some(c) = cells.pop() else {
                    break 'enemy_gen;
                };

                break c;
            };

            map.insert_entity(En::from_template(temp, false, true), nx);
        }
    }
    let mut handle = std::io::stdout();
    execute!(handle, cursor::Hide);

    let mut win_cont = windowed::Container::new();
    let win_left = TERMINAL_WID / 2 - WINDOW_WIDTH / 2;
    let win_top = TERMINAL_HGT / 2 - WINDOW_HEIGHT / 2 - 1;
    win_cont.add_win(windowed::Window::new(Point::new(
        win_left as i32,
        win_top as i32,
    )));
    win_cont.add_win(windowed::Window::new(STATS_POS));
	win_cont.add_win(windowed::Window::new(ATKS_POS));

    let delay = std::time::Duration::from_millis(DELAY);
    let vfx_delay = std::time::Duration::from_millis(VFX_DELAY);
    let mut ready;
	
    // Colours the text with the given colour and puts it into the window. Ensures at least len styled characters
    // are contained within the line.
    let mut add_line = |clr: style::Color,
                        txt: &str,
                        win: &mut windowed::Window<style::StyledContent<char>>,
                        len: usize| {
        let mut line = vec![' '.stylize()];
        for ch in txt.chars() {
            line.push(ch.with(clr));
        }
		let line_len = line.len();
		if line_len < len {
			for _ in 0..len - line_len {
				line.push(' '.stylize());
			}
		}
		
        win.data.push(line);
    };

    // Display the current state of the map into the terminal.
    let mut display_map =
        |map: &bn::Map<En>, win_cont: &mut windowed::Container<style::StyledContent<char>>| {
            let player_pos = unsafe { PLAYER };
            let pl = map.get_ent(player_pos).unwrap();

            // Display the game window.
            let top_left =
                player_pos - Point::new(WINDOW_WIDTH as i32 / 2, -(WINDOW_HEIGHT as i32) / 2);
            let mut cur_win = &mut win_cont.windows[GAME];
            map.display_into(cur_win, top_left, WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32);
            cur_win.outline_with('#'.grey());

            // Create some stats and put them in a window.
            cur_win = &mut win_cont.windows[STATS];
			cur_win.data.clear();
			cur_win.data.push(vec![' '.stylize(); STATS_WID]);
			
			// HP display.
            add_line(
                style::Color::Red,
                &format!("HP: {}/{}", pl.hp.value(), pl.hp.max),
                cur_win,
                STATS_WID as usize,
            );
			// Position display.
            add_line(
                style::Color::Green,
                &format!("{player_pos}"),
                cur_win,
                STATS_WID as usize,
            );
			// Time display.
            add_line(
                style::Color::Blue,
                &format!("Time: {}", unsafe { GLOBAL_TIME }),
                cur_win,
                STATS_WID as usize,
            );
			
			cur_win.data.push(vec![' '.stylize(); STATS_WID]);
			cur_win.outline_with('#'.grey());
			
			// Display current attacks and put them in a window.
            cur_win = &mut win_cont.windows[ATKS];
			cur_win.data.clear();
			let mut damages: HashMap<Point, DmgInst> = HashMap::new();
			
			for atks in pl.atks.melee_atks.values() {
				for atk in atks.iter() {
					for pos in atk.place.iter() {
						damages.insert(*pos, atk.effect);
					}
				}
			}
			
			let win_centre = Point::new((ATKS_WID / 2) as i32, (ATKS_WID / 2) as i32);
			
			for y in 0..ATKS_WID {
				cur_win.data.push(Vec::new());
				for x in 0..ATKS_WID {
					let pos = Point::new(x as i32, y as i32);

					let mut ch = '.'.stylize();
					if pos == win_centre {
						ch = pl.ch;
					} else if let Some(dmg_inst) = damages.get(&(pos - win_centre)) {
						ch = match dmg_inst.dmg {
							DmgType::Dmg(d) => char::from_digit(d, 16).unwrap().red(),
							DmgType::Heal(h) => char::from_digit(h, 16).unwrap().green(),
						};
					}
					cur_win.data[y].push(ch);
				}
				
			}
			
            cur_win.outline_with('#'.grey());

            win_cont.refresh();

            // Print the screen.
            let screen =
                win_cont.to_string_with_default(TERMINAL_WID, TERMINAL_HGT - 1, ' '.stylize());

            queue!(handle, cursor::MoveTo(0, 0), style::Print(screen));

            handle.flush();
        };

    display_map(&map, &mut win_cont);

    'main: loop {
        ready = true;

        if map.get_ent(unsafe { PLAYER }).unwrap().vel.is_none() {
            while let event::Event::Key(ke) = event::read().expect("what") {
                if ke.is_press() {
                    let (mut new, action) = match ke.code {
                        event::KeyCode::Left | event::KeyCode::Char('a') => {
                            (Point::new(-1, 0), ActionType::TryMove)
                        }
                        event::KeyCode::Right | event::KeyCode::Char('d') => {
                            (Point::new(1, 0), ActionType::TryMove)
                        }
                        event::KeyCode::Down | event::KeyCode::Char('s') => {
                            (Point::new(0, -1), ActionType::TryMove)
                        }
                        event::KeyCode::Up | event::KeyCode::Char('w') => {
                            (Point::new(0, 1), ActionType::TryMove)
                        }
                        event::KeyCode::Char('.') => (Point::ORIGIN, ActionType::Wait),
                        event::KeyCode::Char('f') => (Point::ORIGIN, ActionType::Fire(0)),
                        event::KeyCode::Char('g') => (Point::ORIGIN, ActionType::Fire(1)),
                        event::KeyCode::Char('h') => (Point::ORIGIN, ActionType::Fire(2)),
                        event::KeyCode::Enter => break 'main,
                        _ => continue,
                    };

                    // Rotate 45° clockwise if shift is held.
                    if ke.modifiers.intersects(event::KeyModifiers::SHIFT) {
                        new = Point::new(new.x - new.y, new.x + new.y);
                    }

                    unsafe {
                        DIR = new;
                        ACTION = action;
                    }

                    break;
                }
            }
        } else {
            unsafe {
                DIR = Point::ORIGIN;
                ACTION = ActionType::Wait;
            }
            clear_events();
        }

        while map
            .get_highest_priority()
            .map(|(_k, e)| !e.is_player)
            .unwrap_or(false)
            || ready
        {
            ready = false;
            map.update();
            display_map(&map, &mut win_cont);
            // thread::sleep(delay);
            let mut did_vfx = false;
            while map.update_vfx() > 0 {
                did_vfx = true;
                display_map(&map, &mut win_cont);
                thread::sleep(delay);
            }
            display_map(&map, &mut win_cont);
            if did_vfx {
                thread::sleep(vfx_delay);
            }
            unsafe {
                if DEAD {
                    println!("You are dead");
                    break 'main;
                }
            }
        }

        thread::sleep(delay);
    }
}

fn clear_events() {
    while let Ok(b) = event::poll(std::time::Duration::from_secs(0))
        && b
    {
        event::read();
    }
}
