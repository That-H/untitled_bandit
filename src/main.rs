#![allow(unused_must_use)]
#![allow(static_mut_refs)]

use attacks::*;
use bn::windowed;
use crossterm::style::{self, Stylize};
use crossterm::{cursor, event, execute, queue, terminal};
use entity::*;
use io::Write;
use rand::prelude::{IndexedRandom, SliceRandom};
use rand::{Rng, SeedableRng};
use std::{collections::HashMap, io, thread};
use untitled_bandit::*;

const ROOMS: u32 = 10;
const SPECIAL_ROOMS: u32 = 1;
const MAX_WIDTH: i32 = 13;
const MIN_WIDTH: i32 = 6;
const MAP_WIDTH: usize = 300;
const MAP_HEIGHT: usize = 300;
const TERMINAL_WID: u16 = 120;
const TERMINAL_HGT: u16 = 30;
const WINDOW_WIDTH: u16 = 40;
const WINDOW_HEIGHT: u16 = 20;
const ARROWS: [char; 4] = ['↓', '←', '↑', '→'];
// This does look like a key when printed.
const KEY: char = '⚷';
const EXIT_CLRS: [style::Color; 4] = KEY_CLRS;

// All constants below describe the index of the window container that
// the corresponding window is located at, or things about the window.
const GAME: usize = 0;
const STATS: usize = 1;
const STATS_POS: Point = Point::new(22, 4);
const STATS_WID: usize = 15;
const ATKS: usize = 2;
const ATKS_POS: Point = Point::new(28, 12);
const ATKS_WID: usize = 9;
const KEYS: usize = 3;
const KEYS_POS: Point = Point::new(83, 4);
const KEYS_WID: usize = KEY_CLRS.len() * 4 + 1;
const LOG: usize = 4;
const LOG_POS: Point = Point::new(83, 10);
const LOG_WID: usize = 25;
const LOG_HGT: usize = 11;

type StepEffect = dyn Fn(Point, &bn::Map<En>) -> Vec<bn::Cmd<En>>;

fn create_conveyor(disp: Point, revealed: bool) -> Tile {
    let step_effect: Option<Box<StepEffect>> =
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
        locked: None,
    }
}

fn get_exit(revealed: bool, floor_num: usize) -> Tile {
    Tile {
        ch: Some('>'.with(EXIT_CLRS[floor_num])),
        blocking: false,
        empt: false,
        revealed,
        door: None,
        slippery: false,
        step_effect: Some(Box::new(|_, _| {
            unsafe {
                if ENEMIES_REMAINING == 0 {
                    NEXT_FLOOR = true;
                    LOG_MSGS.write().unwrap().clear();
                }
            }
            Vec::new()
        })),
        locked: None,
    }
}

fn get_key(revealed: bool, key_id: u32) -> Tile {
    Tile {
        ch: Some(KEY.with(KEY_CLRS[key_id as usize])),
        blocking: false,
        empt: false,
        revealed,
        door: None,
        slippery: false,
        step_effect: Some(Box::new(move |pos, _| {
            unsafe { KEYS_COLLECTED[key_id as usize] += 1 }
            LOG_MSGS.write().unwrap().push(format!("{} gains key", templates::PLAYER_CHARACTER).into());
            vec![bn::Cmd::new_on(pos).modify_tile(Box::new(|t: &mut Tile| {
                t.step_effect = None;
                t.ch = Some('.'.stylize());
            }))]
        })),
        locked: None,
    }
}

fn main() {
    // Rng used for map generation. Has to be separate to ensure determinism
    // with the map and its contents.
    let mut floor_rng = rand::rngs::StdRng::from_os_rng();

    // Map used through the game.
    let mut map: bn::Map<En> = bn::Map::new(MAP_WIDTH, MAP_HEIGHT);

    // Raw mode required for windowed to work correctly.
    terminal::enable_raw_mode();
    execute!(io::stdout(), terminal::Clear(terminal::ClearType::All));

    let (mut templates, elites) = templates::get_templates();

    // Sort the costs and templates using those costs so that the templates do not have to be returned in sorted order.
    // e, h, l, k, b, w
    let costs = HashMap::from([
        ('e', 12),
        ('h', 20),
        ('l', 45),
        ('k', 37),
        ('b', 34),
        ('w', 40),
        ('o', 15),
        ('v', 45),
        ('B', 50),
    ]);
    templates.sort_by_key(|temp| costs[temp.ch.content()]);

    let get_temp = |budget: u32,
                    rng: &mut rand::rngs::StdRng,
                    elite: bool|
     -> Option<(&EntityTemplate, u32)> {
        let temps = if elite { &elites } else { &templates };
        let possible: Vec<_> = temps
            .iter()
            .filter_map(|t| {
                let cost = costs[t.ch.content()];
                if cost <= budget {
                    Some((t, cost))
                } else {
                    None
                }
            })
            .collect();
        possible.choose(rng).cloned()
    };

    let gen_floor = |map: &mut bandit::Map<En>, rng: &mut rand::rngs::StdRng, floor_num: u32| {
        // Create the player if it is the first floor, otherwise get them.
        let pl = if floor_num == 0 {
            templates::get_player()
        } else {
            map.get_ent(unsafe { PLAYER }).unwrap().clone()
        };

        // Reinitialise the map.
        *map = bandit::Map::new(0, 0);

        unsafe { PLAYER = Point::ORIGIN }
        map.insert_entity(pl, unsafe { PLAYER });

        // Generate the rooms of the map.
        let (mut grid, mut rooms) =
            map_gen::map_gen(ROOMS - SPECIAL_ROOMS, MAX_WIDTH, MIN_WIDTH, rng);

        // Whether we have chosen an exit room yet.
        let mut done_exit = false;
        // Location of the door to the exit room.
        let mut true_door = Point::ORIGIN;
        // Rect id of the exit room so that the key room does not generate off of it.
        let mut exit_id: usize = 0;

        // Generate enemies.
        for (n, r) in rooms.iter().enumerate().skip(1) {
            let mut budget = (r.wid * r.hgt) as u32 / 2;
            let mut cells: Vec<Point> = r.inner_cells().collect();
            cells.shuffle(rng);
            let mut elite = false;

            // Eligible to be an exit room if there is one door to it.
            if !done_exit {
                for pos in r.edges() {
                    if let map_gen::Cell::Door(_, _) = grid.get(&pos).unwrap() {
                        elite = !elite;
                        true_door = pos;
                        exit_id = n;
                        if !elite {
                            break;
                        }
                    }
                }
            }

            if elite {
                done_exit = true;
                budget = 75;
                map.insert_tile(
                    get_exit(false, floor_num as usize),
                    r.top_left() + Point::new(r.wid / 2, -r.hgt / 2),
                );
            }

            'enemy_gen: while let Some((temp, cost)) = get_temp(budget, rng, elite) {
                budget -= cost;
                // Exit early if there is no where to place the entity.
                let Some(nx) = cells.pop() else {
                    break 'enemy_gen;
                };

                map.insert_entity(En::from_template(temp, false, true), nx);
            }
        }

        // Generate a new room specifically for the key.
        map_gen::gen_rect_in(
            &mut rooms,
            &mut grid,
            rng,
            MIN_WIDTH,
            MAX_WIDTH,
            &[0, exit_id],
        );
        let key_pos = *rooms
            .last()
            .unwrap()
            .inner_cells()
            .collect::<Vec<Point>>()
            .choose(rng)
            .unwrap();

        // Put the cells actually into the map.
        for y in -(((MAP_HEIGHT / 2) + MAP_OFFSET) as i32)..(MAP_HEIGHT / 2 + MAP_OFFSET) as i32 {
            for x in -(((MAP_WIDTH / 2) + MAP_OFFSET) as i32)..(MAP_WIDTH / 2 + MAP_OFFSET) as i32 {
                let pos = Point::new(x, y);
                let mut blocking = false;
                let mut door = None;

                // If there is already a tile there, don't overwrite it.
                if map.get_map(pos).is_some() {
                    continue;
                };

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
                            door = Some((rooms[*id1], rooms[*id2]));
                            Some('/'.yellow())
                        }
                    },
                    None => None,
                };
                let revealed = rooms[0].contains(Point::new(x, y));
                let t = if rng.random_bool(0.0) && !blocking && door.is_none() {
                    create_conveyor(
                        *Point::ORIGIN.get_all_adjacent().choose(rng).unwrap(),
                        revealed,
                    )
                } else if pos == key_pos {
                    get_key(false, floor_num)
                } else {
                    Tile {
                        ch,
                        blocking,
                        empt: false,
                        revealed,
                        slippery: false,
                        door,
                        step_effect: None,
                        locked: None,
                    }
                };

                map.insert_tile(t, pos);
            }
        }

        let door = map.get_map_mut(true_door).unwrap();
        door.ch = Some('╬'.with(KEY_CLRS[floor_num as usize]));
        door.locked = Some(floor_num);
        door.blocking = true;
    };

    gen_floor(&mut map, &mut floor_rng, unsafe { FLOORS_CLEARED });

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
    win_cont.add_win(windowed::Window::new(KEYS_POS));
    win_cont.add_win(windowed::Window::new(LOG_POS));

    let delay = std::time::Duration::from_millis(DELAY);
    let vfx_delay = std::time::Duration::from_millis(VFX_DELAY);
    let mut ready;

    // Colours the text with the given colour and puts it into the window. Ensures at least len styled characters
    // are contained within the line.
    let add_line = |clr: style::Color,
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
                STATS_WID,
            );
            // Floor display.
            add_line(
                style::Color::Green,
                &format!("Floor {}", unsafe { FLOORS_CLEARED }),
                cur_win,
                STATS_WID,
            );
            // Position display.
            add_line(
                style::Color::Green,
                &format!("{player_pos}"),
                cur_win,
                STATS_WID,
            );
            // Time display.
            add_line(
                style::Color::Blue,
                &format!("Time: {}", unsafe { GLOBAL_TIME }),
                cur_win,
                STATS_WID,
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
                        for ef in atk.effects.iter() {
                            if let Effect::DoDmg(dmg_inst) = ef {
                                damages.insert(*pos, *dmg_inst);
                            }
                        }
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

            // Inform the player of their current held keys.
            cur_win = &mut win_cont.windows[KEYS];
            cur_win.data.clear();
            cur_win.data.push(vec![' '.stylize(); KEYS_WID]);

            add_line(style::Color::White, "KEYS:", cur_win, KEYS_WID);
            let mut next_line = Vec::new();
            for (n, clr) in KEY_CLRS.iter().enumerate() {
                next_line.push(' '.stylize());
                let keys = unsafe { KEYS_COLLECTED[n] };
                next_line.push(char::from_digit(keys, 16).unwrap().stylize());
                next_line.push('x'.stylize());
                next_line.push(
                    KEY.with(if keys > 0 {
                        *clr
                    } else {
                        style::Color::DarkGrey
                    }),
                );
            }

            next_line.push(' '.stylize());
            cur_win.data.push(next_line);
            cur_win.data.push(vec![' '.stylize(); KEYS_WID]);

            cur_win.outline_with('#'.grey());

            // Tell the player the last few things that have occurred.
            cur_win = &mut win_cont.windows[LOG];
            cur_win.data.clear();
            cur_win.data.push(vec![' '.stylize(); LOG_WID]);
            add_line(style::Color::White, "LOG: ", cur_win, LOG_WID);
            let read = LOG_MSGS.read().unwrap();
            let len = read.len();
            let start = if len < LOG_HGT {
                0
            } else {
                len - LOG_HGT
            };

            for msg in LOG_MSGS.read().unwrap()[start..len].iter() {
                add_line(style::Color::White, &msg.to_string(), cur_win, LOG_WID);
            }

            for _ in cur_win.data.len()..=LOG_HGT + 1 {
                cur_win.data.push(vec![' '.stylize(); LOG_WID]);
            }

            cur_win.data.push(vec![' '.stylize(); LOG_WID]);
            cur_win.outline_with('#'.grey());

            win_cont.refresh();

            // Print the screen.
            let screen =
                win_cont.to_string_with_default(TERMINAL_WID, TERMINAL_HGT - 1, ' '.stylize());

            for (y, line) in screen.lines().enumerate() {
                queue!(handle, cursor::MoveTo(0, y as u16), style::Print(line));
            }

            handle.flush();
        };

    display_map(&map, &mut win_cont);

    'main: loop {
        ready = true;

        if map.get_ent(unsafe { PLAYER }).unwrap().vel.is_none() {
            while let event::Event::Key(ke) = event::read().expect("what") {
                if ke.is_press() {
                    let action = match ke.code {
                        // Has arrow keys, wasd, and, for the vim users among us, hjkl.
                        event::KeyCode::Left
                        | event::KeyCode::Char('a')
                        | event::KeyCode::Char('h') => ActionType::TryMove(Point::new(-1, 0)),
                        event::KeyCode::Right
                        | event::KeyCode::Char('d')
                        | event::KeyCode::Char('l') => ActionType::TryMove(Point::new(1, 0)),
                        event::KeyCode::Down
                        | event::KeyCode::Char('s')
                        | event::KeyCode::Char('j') => ActionType::TryMove(Point::new(0, -1)),
                        event::KeyCode::Up
                        | event::KeyCode::Char('w')
                        | event::KeyCode::Char('k') => ActionType::TryMove(Point::new(0, 1)),
                        event::KeyCode::Char('.') => ActionType::Wait,
                        event::KeyCode::Char('f') => ActionType::Fire(0),
                        event::KeyCode::Char('g') => ActionType::Fire(1),
                        event::KeyCode::Char('b') => ActionType::Fire(2),
                        event::KeyCode::Enter => break 'main,
                        _ => continue,
                    };

                    unsafe {
                        ACTION = action;
                    }

                    break;
                }
            }
        } else {
            unsafe {
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
                // Check if the player has died.
                if map.get_ent(PLAYER).unwrap().is_dead() {
                    println!("you are dead");
                    break 'main;
                }

                // Check if the player has left the floor.
                if NEXT_FLOOR {
                    FLOORS_CLEARED += 1;
                    NEXT_FLOOR = false;
                    gen_floor(&mut map, &mut floor_rng, FLOORS_CLEARED);
                    display_map(&map, &mut win_cont);
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
