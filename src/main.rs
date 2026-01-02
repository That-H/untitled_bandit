#![allow(unused_must_use)]
#![allow(static_mut_refs)]

use attacks::*;
use bn::windowed;
use crossterm::style::{self, Stylize};
use crossterm::{cursor, event, execute, queue, terminal};
use entity::*;
use io::{Read, Write};
use rand::{Rng, SeedableRng};
use std::{collections::HashMap, env, fs, io, thread, time};
use untitled_bandit::*;
use tile_presets::*;
use map_gen::bandit_gen::*;

// Directory of the assets.
const ASSETS_DIR: &str = "assets";

// UI constants.
const SELECTOR: &str = ">";
const SELECTOR_CLR: style::Color = style::Color::Rgb {
    r: 255,
    g: 240,
    b: 0,
};
const HOVER_CLR: style::Color = style::Color::Rgb {
    r: 255,
    g: 240,
    b: 0,
};

const TERMINAL_WID: u16 = 120;
const TERMINAL_HGT: u16 = 30;
const WINDOW_WIDTH: u16 = 40;
const WINDOW_HEIGHT: u16 = 20;

// All constants below describe the index of the window container that
// the corresponding window is located at, or things about the window.
const GAME: usize = 0;
const STATS: usize = 1;
const STATS_POS: Point = Point::new(22, 4);
const STATS_WID: usize = 15;
const ATKS: usize = 2;
const ATKS_POS: Point = Point::new(32, 12);
const ATKS_WID: usize = 5;
const KEYS: usize = 3;
const KEYS_POS: Point = Point::new(83, 4);
const KEYS_WID: usize = KEY_CLRS.len() * 4 + 1;
const LOG: usize = 4;
const LOG_POS: Point = Point::new(83, 10);
const LOG_WID: usize = 29;
const LOG_HGT: usize = 11;
const DEBUG_WIN: usize = 5;
const DEBUG_POS: Point = Point::new(13, 19);
const DEBUG_WID: usize = 24;
const SEED_WIN: usize = 6;
const SEED_POS: Point = Point::new(13, 19);
const SEED_WID: usize = 24;

// Events for the ui.
const QUIT: u32 = 0;
const MAIN_MENU: u32 = 1;
const QUICK_RESET: u32 = 2;
const PLAY: u32 = 3;

// Seed.
static mut SEED: u64 = 0x3213CA29C823B78A;

// Whether cheats are enabled. Only possible in a debug build.
const CHEATS: bool = if cfg!(debug_assertions) { true } else { false };
// Whether this here initial seed should be ignored.
const SEED_OVERRIDE: bool = !CHEATS;

fn main() {
    // Get the path to this executable so that assets can be loaded even if the project is
    // downloaded from github.
    let mut this_path = env::current_exe().expect("Failed to get path to project");
    for _ in 0..3 {
        this_path.pop();
    }
    this_path.push(ASSETS_DIR);

    // Rng used for map generation. Has to be separate to ensure determinism
    // with the map and its contents.
    let mut floor_rng;

    // Raw mode required for windowed to work correctly.
    terminal::enable_raw_mode();
    execute!(io::stdout(), terminal::Clear(terminal::ClearType::All));

    let meta = templates::metadata::get_metadata();
    let (templates, elites) = templates::get_templates();

    // Contains additional metadata about each enemy type.
    let mut handle = std::io::stdout();
    execute!(handle, cursor::Hide);

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

    // Display the given window container to the screen.
    let print_win = |win_cont: &windowed::Container<style::StyledContent<char>>| {
        let mut handle = io::stdout();

        // Print the screen.
        let screen = win_cont.to_string_with_default(TERMINAL_WID, TERMINAL_HGT - 1, ' '.stylize());

        for (y, line) in screen.lines().enumerate() {
            queue!(handle, cursor::MoveTo(0, y as u16), style::Print(line));
        }

        handle.flush();
    };

    // Display the current state of the map into the terminal.
    let display_map =
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
                next_line.push(KEY.with(if keys > 0 {
                    *clr
                } else {
                    style::Color::DarkGrey
                }));
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
            let start = if len < LOG_HGT { 0 } else { len - LOG_HGT };

            for msg in LOG_MSGS.read().unwrap()[start..len].iter() {
                add_line(style::Color::White, &msg.to_string(), cur_win, LOG_WID);
            }

            for _ in cur_win.data.len()..=LOG_HGT + 1 {
                cur_win.data.push(vec![' '.stylize(); LOG_WID]);
            }

            cur_win.data.push(vec![' '.stylize(); LOG_WID]);
            cur_win.outline_with('#'.grey());

            if cfg!(debug_assertions) {
                // Display some debug information.
                cur_win = &mut win_cont.windows[DEBUG_WIN];
                cur_win.data.clear();
                cur_win.data.push(vec![' '.stylize(); DEBUG_WID]);

                let cur_seed = unsafe { SEED };
                add_line(
                    style::Color::White,
                    &format!("SEED: {cur_seed:X} "),
                    cur_win,
                    DEBUG_WID,
                );
                add_line(
                    style::Color::White,
                    &format!("Enemies: {}", unsafe { ENEMIES_REMAINING }),
                    cur_win,
                    DEBUG_WID,
                );
                add_line(
                    style::Color::White,
                    &format!("NoClip: {}", if *NO_CLIP.read().unwrap() { "yes" } else { "nah" }),
                    cur_win,
                    DEBUG_WID,
                );

                cur_win.data.push(vec![' '.stylize(); DEBUG_WID]);

                cur_win.outline_with('#'.grey());
            } else {
                // Display just the seed.
                cur_win = &mut win_cont.windows[SEED_WIN];
                cur_win.data.clear();

                let cur_seed = unsafe { SEED };
                add_line(
                    style::Color::White,
                    &format!("Seed: {cur_seed:X} "),
                    cur_win,
                    SEED_WID,
                );

                cur_win.outline_with('#'.grey());
            }

            win_cont.refresh();

            print_win(win_cont);
        };

    // True if the main_menu should be skipped.
    let mut quick_restart = false;

    'full: loop {
        // Reset globals.
        unsafe {
            PLAYER = Point::ORIGIN;
            GLOBAL_TIME = 0;

            // Give a lot of keys on a debug build.
            let key_count = if CHEATS { 9 } else { 0 };
            KEYS_COLLECTED = [key_count; entity::KEY_CLRS_COUNT];
            LOG_MSGS.write().unwrap().clear();
            LAST_DOOR.write().unwrap().take();
            DEAD = false;
            FLOORS_CLEARED = 0;
            NEXT_FLOOR = false;
            ENEMIES_REMAINING = 0;
            ACTION = ActionType::Wait;
            KILLED = 0;
            // Reseed the rng if we want to override the original one.
            if SEED_OVERRIDE {
                SEED = rand::rng().random();
            }
            floor_rng = rand::rngs::SmallRng::seed_from_u64(SEED as u64);
        }
        let delay = time::Duration::from_millis(DELAY);
        let vfx_delay = time::Duration::from_millis(VFX_DELAY);
        let mut ready;

        // Main menu here.
        let mut main_menu_cont: windowed::Container<style::StyledContent<char>> =
            windowed::Container::new();

        // Title text.
        main_menu_cont.add_win(windowed::Window::new(Point::new(26, 1)));

        if !quick_restart {
            // Open the main menu file.
            let mut f = fs::File::open(this_path.join("main_menu.txt")).unwrap();
            let mut main_text = String::new();
            f.read_to_string(&mut main_text);

            for line in main_text.lines() {
                add_line(
                    style::Color::White,
                    line,
                    &mut main_menu_cont.windows[0],
                    128,
                );
                main_menu_cont.refresh();
                print_win(&main_menu_cont);
                thread::sleep(delay);
            }
        }

        clear_events();

        let mut menu_container = ui::UiContainer::new();

        // Main menu.
        let mut scene = ui::Scene::new(Point::new(56, 20), 8, 4);

        let basic_button = ui::widgets::Button::empty_new()
            .set_selector(String::from(SELECTOR))
            .set_hover_clr(HOVER_CLR)
            .set_selector_clr(SELECTOR_CLR)
            .set_static_len(true);

        scene.add_element(
            Box::new(
                basic_button
                    .clone()
                    .set_txt(String::from("Play"))
                    .set_event(ui::Event::Exit(PLAY))
                    .set_screen_pos(Point::new(1, 1)),
            ),
            Point::new(1, 1),
        );
        scene.add_element(
            Box::new(
                basic_button
                    .clone()
                    .set_txt(String::from("Quit"))
                    .set_event(ui::Event::Exit(QUIT))
                    .set_screen_pos(Point::new(1, 2)),
            ),
            Point::new(1, 2),
        );
        scene.add_element(
            Box::new(ui::widgets::Outline::new('#'.grey(), 8)),
            Point::new(999, 999),
        );
        scene.move_cursor(Point::new(1, 1));
        menu_container.add_scene(scene);

        // Death / win_screen.
        let mut end_scene = ui::Scene::new(Point::new(54, 20), 12, 5);

        end_scene.add_element(
            Box::new(
                basic_button
                    .clone()
                    .set_txt(String::from("New run"))
                    .set_event(ui::Event::Exit(QUICK_RESET))
                    .set_screen_pos(Point::new(1, 1)),
            ),
            Point::new(1, 1),
        );
        end_scene.add_element(
            Box::new(
                basic_button
                    .clone()
                    .set_txt(String::from("Main Menu"))
                    .set_event(ui::Event::Exit(MAIN_MENU))
                    .set_screen_pos(Point::new(1, 2)),
            ),
            Point::new(1, 2),
        );
        end_scene.add_element(
            Box::new(
                basic_button
                    .clone()
                    .set_txt(String::from("Quit"))
                    .set_event(ui::Event::Exit(QUIT))
                    .set_screen_pos(Point::new(1, 3)),
            ),
            Point::new(1, 3),
        );
        end_scene.add_element(
            Box::new(ui::widgets::Outline::new('#'.grey(), 12)),
            Point::new(999, 999),
        );
        end_scene.move_cursor(Point::new(1, 1));

        menu_container.add_scene(end_scene);

        if !quick_restart {
            match menu_container.run() {
                QUIT => break 'full,
                PLAY => (),
                c => panic!("Unexpected code '{c}'"),
            }
        }
        quick_restart = false;

        // Time when the game began.
        let start = time::Instant::now();

        // Create the various windows required for the main game.
        let mut main_wins = windowed::Container::new();
        let win_left = TERMINAL_WID / 2 - WINDOW_WIDTH / 2;
        let win_top = TERMINAL_HGT / 2 - WINDOW_HEIGHT / 2 - 1;
        main_wins.add_win(windowed::Window::new(Point::new(
            win_left as i32,
            win_top as i32,
        )));
        main_wins.add_win(windowed::Window::new(STATS_POS));
        main_wins.add_win(windowed::Window::new(ATKS_POS));
        main_wins.add_win(windowed::Window::new(KEYS_POS));
        main_wins.add_win(windowed::Window::new(LOG_POS));
        main_wins.add_win(windowed::Window::new(DEBUG_POS));
        main_wins.add_win(windowed::Window::new(SEED_POS));

        // Map used through the game.
        let mut map: bn::Map<En> = bn::Map::new(69, 69);

        // Generate the initial floor.
        gen_floor(&mut map, &mut floor_rng, unsafe { FLOORS_CLEARED }, &meta, &templates, &elites);

        display_map(&map, &mut main_wins);

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
                            // Skip to next floor.
                            event::KeyCode::Char('n') => {
                                unsafe {
                                    if CHEATS {
                                        NEXT_FLOOR = true;
                                        ENEMIES_REMAINING = 0;
                                        if FLOORS_CLEARED + 1 == KILL_SCREEN as u32 {
                                            break 'main;
                                        }
                                    }
                                }
                                continue;
                            }
                            // Turn on no clip.
                            event::KeyCode::Char('c') => {
                                if CHEATS {
                                    let clipping = *NO_CLIP.read().unwrap();
                                    *NO_CLIP.write().unwrap() = !clipping;

                                    let mut write = LOG_MSGS.write().unwrap();
                                    write.push(LogMsg::new(format!(
                                        "{} {}s hacking",
                                        templates::PLAYER_CHARACTER,
                                        if clipping { "stop" } else { "start" }
                                    )));
                                    ActionType::Wait
                                } else {
                                    continue;
                                }
                            }
                            // Change seed and restart quickly.
                            #[cfg(debug_assertions)]
                            event::KeyCode::Char('x') => unsafe {
                                SEED = rand::rng().random();
                                ENEMIES_REMAINING = 0;
                                quick_restart = true;
                                continue 'full;
                            },
                            #[cfg(debug_assertions)]
                            event::KeyCode::Char('v') => {
                                check_seeds(0x5F19E7B2F1F16EAB, 10);
                                ActionType::Wait
                            },
                            // Kill everyone in the room.
                            event::KeyCode::Char('*') => {
                                unsafe {
                                    if CHEATS {
                                        let mut dead = Vec::new();

                                        for (&pos, _en) in map.get_entities() {
                                            if pos != PLAYER {
                                                dead.push(pos);
                                            }
                                        }

                                        for d in dead {
                                            let e = map.get_ent_mut(d).unwrap();
                                            if !e.dormant {
                                                e.hp.set_to(0);
                                            }
                                        }

                                        let mut write = LOG_MSGS.write().unwrap();
                                        write.push(LogMsg::new(format!(
                                            "{} inquires about the",
                                            templates::PLAYER_CHARACTER
                                        )));
                                        write.push(LogMsg::new(String::from(
                                            "extended warranty of",
                                        )));
                                        write.push(LogMsg::new(String::from(
                                            "the enemies' vehicles",
                                        )));
                                        ActionType::Wait
                                    } else {
                                        continue;
                                    }
                                }
                            }
                            event::KeyCode::Char('r') => {
                                let read = LAST_DOOR.read().unwrap();
                                let disp = unsafe {
                                    let old = PLAYER;
                                    if ENEMIES_REMAINING == 0 {
                                        if let Some(p) = *read
                                            && p != Point::ORIGIN
                                        {
                                            PLAYER = p;
                                            p - old
                                        } else {
                                            Point::ORIGIN
                                        }
                                    } else {
                                        Point::ORIGIN
                                    }
                                };
                                ActionType::TryMove(disp)
                            }
                            event::KeyCode::Esc => {
                                unsafe {
                                    DEAD = true;
                                }
                                break 'main;
                            }
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
                display_map(&map, &mut main_wins);
                // thread::sleep(delay);
                let mut did_vfx = false;
                while map.update_vfx() > 0 {
                    did_vfx = true;
                    display_map(&map, &mut main_wins);
                    thread::sleep(delay);
                }
                display_map(&map, &mut main_wins);
                if did_vfx {
                    thread::sleep(vfx_delay);
                }
                unsafe {
                    // Check if the player has died.
                    if map.get_ent(PLAYER).unwrap().is_dead() {
                        DEAD = true;
                        break 'main;
                    }

                    // Check if the player has left the floor.
                    if NEXT_FLOOR {
                        FLOORS_CLEARED += 1;
                        if FLOORS_CLEARED == KILL_SCREEN as u32 {
                            break 'main;
                        }
                        NEXT_FLOOR = false;
                        gen_floor(&mut map, &mut floor_rng, FLOORS_CLEARED, &meta, &templates, &elites);
                        display_map(&map, &mut main_wins);
                    }
                }
            }

            thread::sleep(delay);
        }
        // Death/win screen.
        let mut end_wins = windowed::Container::new();

        let main_wid = 38;
        let time_taken = time::Instant::now().duration_since(start).as_secs();
        let (fname, txt_pos) = if unsafe { DEAD } {
            ("death.txt", Point::new(3, 2))
        } else {
            ("win.txt", Point::new(26, 2))
        };
        end_wins.add_win(windowed::Window::new(txt_pos));
        end_wins.add_win(windowed::Window::new(Point::new(40, 12)));

        // Open the relevant file.
        let mut f = fs::File::open(this_path.join(fname)).unwrap();
        let mut text = String::new();
        f.read_to_string(&mut text);

        for line in text.lines() {
            add_line(style::Color::White, line, &mut end_wins.windows[0], 128);
            end_wins.refresh();
            print_win(&end_wins);
            thread::sleep(delay);
        }

        clear_events();
        let cur_win = &mut end_wins.windows[1];

        add_line(style::Color::White, "", cur_win, main_wid);

        // Real time taken.
        add_line(
            style::Color::White,
            &format!("Time Elapsed: {}:{:02}", time_taken / 60, time_taken % 60,),
            cur_win,
            main_wid,
        );

        // In game time taken.
        add_line(
            style::Color::White,
            &format!("Turns: {}", unsafe { GLOBAL_TIME },),
            cur_win,
            main_wid,
        );

        // Floor reached.
        add_line(
            style::Color::White,
            &format!("Floor Reached: {}", unsafe { FLOORS_CLEARED },),
            cur_win,
            main_wid,
        );

        // Enemies killed.
        add_line(
            style::Color::White,
            &format!("Enemies Killed: {}", unsafe { KILLED },),
            cur_win,
            main_wid,
        );

        add_line(style::Color::White, "", cur_win, main_wid);

        cur_win.outline_with('#'.stylize());
        thread::sleep(time::Duration::from_millis(275));
        end_wins.refresh();
        print_win(&end_wins);

        menu_container.change_scene(1);
        match menu_container.run() {
            QUIT => break 'full,
            MAIN_MENU => (),
            QUICK_RESET => quick_restart = true,
            c => panic!("Unexpected code '{c}'"),
        }
    }
}

fn clear_events() {
    while let Ok(b) = event::poll(time::Duration::from_secs(0))
        && b
    {
        event::read();
    }
}

/// Checks some seeds for suspicousness. Returns true if any are sus.
#[cfg(debug_assertions)]
fn check_seeds(init_seed: u64, sds: u64) -> bool {
    let meta = templates::metadata::get_metadata();
    let (templates, elites) = templates::get_templates();
    let mut map = bn::Map::new(69, 69);

    let mut found_fault = false;

    for sd in init_seed..init_seed+sds {
        let mut floor_rng = rand::rngs::SmallRng::seed_from_u64(sd);
        eprint!("Trying {sd:X}");
        eprint!("\r");
        gen_floor(&mut map, &mut floor_rng, 0, &meta, &templates, &elites);
        let test = |t: Option<&Tile>| {
            if let Some(t) = t
                && t.door.is_some()
            {
                true
            } else {
                false
            }
        };
        let depth = 25;
        for y in -depth..=depth {
            for x in -depth..=depth {
                let pr = Point::new(x, y);
                let hell_ps =
                    [Point::new(x - 1, y), Point::new(x, y - 1)];
                if test(map.get_map(pr)) {
                    let mut wall_count = 0;
                    for dis in Point::ORIGIN.get_all_adjacent() {
                        match map.get_map(pr + dis) {
                            Some(t) => {
                                if t.blocking {
                                    wall_count += 1
                                }
                            }
                            None => continue,
                        }
                    }

                    match wall_count {
                        0 | 1 => { 
                            found_fault = true;
                            eprintln!("{sd:X} at {pr} sus door");
                        }
                        3 | 4 => {
                            eprintln!("{sd:X} at {pr} impassable door");
                            found_fault = true;
                        }
                        _ => (),
                    }

                    if hell_ps.into_iter().all(|p| test(map.get_map(p)))
                    {
                        found_fault = true;
                        eprintln!("{sd:X} at {pr} door hell");
                    }
                }
            }
        }
    }

    if !found_fault {
        // Clear the line if everything is a-ok
        eprintln!("\r")
    }

    found_fault
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_tester() {
        let sds = 1024;

        let init_seed = rand::rng().random_range(0..u64::MAX-sds);
        let found_fault = check_seeds(init_seed, sds);
        if found_fault {
            panic!();
        }
    }
}
