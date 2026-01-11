#![allow(unused_must_use)]
#![allow(static_mut_refs)]

use attacks::*;
use bn::windowed;
use crossterm::style::{self, Stylize};
use crossterm::{cursor, event, execute, queue, terminal};
use entity::*;
use io::{Read, Write};
use map_gen::bandit_gen::*;
use rand::{Rng, SeedableRng};
use std::{collections::HashMap, env, fs, io, thread, time};
use tile_presets::*;
use untitled_bandit::*;

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
const PUZZLE_WIN: usize = 7;
const PUZZLE_WID: usize = 16;
const PUZZLE_POS: Point = Point::new(TERMINAL_WID as i32 / 2 - PUZZLE_WID as i32 / 2, 0);

// Events for the ui.
const QUIT: u32 = 0;
const MAIN_MENU: u32 = 1;
const QUICK_RESET: u32 = 2;
const PLAY: u32 = 3;
const PLAY_SEEDED: u32 = 4;
const PUZZLE_SELECT: u32 = 5;
const NEXT_PUZZLE: u32 = 6;

// Seed.
static mut SEED: u64 = 0xBB219F0909BD0E20;

// Contains the id of the puzzle if we are currently doing one. Not to be confused with a puzzle room.
static mut PUZZLE: Option<usize> = None;

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
    execute!(
        io::stdout(),
        terminal::Clear(terminal::ClearType::All),
        terminal::SetSize(TERMINAL_WID, TERMINAL_HGT),
    );

    // Get entity templates.
    let meta = templates::metadata::get_metadata();
    let (templates, elites) = templates::get_templates();

    // Load puzzles.
    let mut tile_set = puzzle_loader::ts::TileSet::new();
    tile_set.add_temps(&templates);
    tile_set.add_temps(&elites);

    // Add the player with 1 hp.
    let mut pzl_player = templates::get_player();
    pzl_player.hp.change_max(1);
    tile_set.add_entity(pzl_player);

    // Add some keys
    for i in 0..KILL_SCREEN {
        tile_set.add_tile(tile_presets::get_key(true, i as u32));
    }

    // Add the exit tile.
    tile_set.add_tile(tile_presets::get_exit(true, 0));

    // Add the walls.
    tile_set.add_tile(Tile {
        ch: Some('#'.grey()),
        empt: false,
        blocking: true,
        door: None,
        revealed: true,
        locked: None,
        slippery: false,
        step_effect: None,
    });

    let empty_t = Tile {
        ch: Some('.'.grey()),
        empt: false,
        blocking: false,
        door: None,
        revealed: true,
        locked: None,
        slippery: false,
        step_effect: None,
    };

    // Add the floor.
    tile_set.add_tile(empty_t.clone());
    // Add a slippery floor.
    tile_set.add_tile(
        Tile {
            slippery: true,
            ch: Some(ICE_CHAR.with(ICE_CLR)),
            ..empty_t.clone()
        }
    );

    // Load in the completion state of the puzzles.
    let stars_read = puzzle_loader::pzl_save::load_pzl_save();
    let pzls =
        match puzzle_loader::load_pzls(this_path.join("puzzles.txt"), &empty_t, &tile_set) {
            Ok(pzls) => pzls,
            Err(why) => panic!("{why}"),
        };
    let mut stars_earned = HashMap::new();
    
    // Discard any data about non existent puzzles.
    for pzl in &pzls {
        stars_earned.insert(pzl.id, if let Some(&hsh) = stars_read.get(&pzl.id) {hsh} else {0});
    }

    let pzl_count = pzls.len();

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
            let is_puzzle = unsafe { PUZZLE.is_some() };

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
            if unsafe { PUZZLE.is_none() } {
                // Floor display.
                add_line(
                    style::Color::Green,
                    &format!("Floor {}", unsafe { FLOORS_CLEARED }),
                    cur_win,
                    STATS_WID,
                );
            }
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
            let start = len.saturating_sub(LOG_HGT);

            for msg in LOG_MSGS.read().unwrap()[start..len].iter() {
                add_line(style::Color::White, &msg.to_string(), cur_win, LOG_WID);
            }

            for _ in cur_win.data.len()..=LOG_HGT + 1 {
                cur_win.data.push(vec![' '.stylize(); LOG_WID]);
            }

            cur_win.data.push(vec![' '.stylize(); LOG_WID]);
            cur_win.outline_with('#'.grey());

            if !is_puzzle {
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
                        &format!(
                            "NoClip: {}",
                            if *NO_CLIP.read().unwrap() {
                                "yes"
                            } else {
                                "nah"
                            }
                        ),
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
            } else {
                // Display just the seed.
                cur_win = &mut win_cont.windows[PUZZLE_WIN];
                cur_win.data.clear();

                let cur_puz = unsafe { *PUZZLE.as_ref().unwrap() };
                add_line(
                    style::Color::White,
                    &format!("{}", pzls[cur_puz].diff),
                    cur_win,
                    PUZZLE_WID,
                );
                add_line(
                    style::Color::White,
                    &format!("Puzzle {}", cur_puz+1),
                    cur_win,
                    PUZZLE_WID,
                );

                cur_win.outline_with('#'.grey());
            }

            win_cont.refresh();

            print_win(win_cont);
        };

    // True if the main_menu should be skipped.
    let mut quick_restart = false;

    // True if we should go straight to the puzzle screen instead of the main menu.
    let mut insta_puzzle = false;

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
            unsafe {
                PUZZLE = None;
            }
            // Open the main menu file.
            let mut f = fs::File::open(this_path.join("main_menu.txt")).unwrap();
            let mut main_text = String::new();
            f.read_to_string(&mut main_text);

            for line in main_text.lines() {
                add_line(
                    style::Color::White,
                    line,
                    &mut main_menu_cont.windows[0],
                    TERMINAL_WID as usize,
                );
                main_menu_cont.refresh();
                print_win(&main_menu_cont);
                thread::sleep(delay);
            }
        }

        clear_events();

        let mut menu_container = ui::UiContainer::new();

        // Main menu.
        let mut scene = ui::Scene::new(Point::new(52, 20), 16, 6);

        let basic_button = ui::widgets::Button::empty_new()
            .set_selector(String::from(SELECTOR))
            .set_hover_clr(HOVER_CLR)
            .set_selector_clr(SELECTOR_CLR)
            .set_static_len(true);

        let basic_entry = ui::widgets::TextEntry::new()
            .set_hover_clr(HOVER_CLR)
            .set_highlight_clr(style::Color::Cyan)
            .set_active_clr(HOVER_CLR);

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
                    .set_txt(String::from("Seeded Run"))
                    .set_event(ui::Event::ChangeScene(1))
                    .set_screen_pos(Point::new(1, 2)),
            ),
            Point::new(1, 2),
        );
        scene.add_element(
            Box::new(
                basic_button
                    .clone()
                    .set_txt(String::from("Puzzles"))
                    .set_event(ui::Event::ChangeScene(3))
                    .set_screen_pos(Point::new(1, 3)),
            ),
            Point::new(1, 3),
        );
        scene.add_element(
            Box::new(
                basic_button
                    .clone()
                    .set_txt(String::from("Save and Quit"))
                    .set_event(ui::Event::Exit(QUIT))
                    .set_screen_pos(Point::new(1, 4)),
            ),
            Point::new(1, 4),
        );
        scene.add_element(
            Box::new(ui::widgets::Outline::new('#'.grey(), 16)),
            Point::new(999, 999),
        );
        scene.move_cursor(Point::new(1, 1));
        menu_container.add_scene(scene);

        // Seed entry screen.
        let seed_wid = 20;
        let mut seed_scene = ui::Scene::new(
            Point::new(TERMINAL_WID as i32 / 2 - seed_wid as i32 / 2, 20),
            seed_wid,
            5,
        );

        seed_scene.add_element(
            Box::new(
                basic_button
                    .clone()
                    .set_txt(String::from("Enter Seed: "))
                    .set_screen_pos(Point::new(1, 1)),
            ),
            Point::new(0, 0),
        );
        seed_scene.add_element(
            Box::new(
                basic_entry
                    .clone()
                    .set_len(16)
                    .set_screen_pos(Point::new(2, 2)),
            ),
            Point::new(1, 2),
        );
        seed_scene.add_element(
            Box::new(
                basic_button
                    .clone()
                    .set_txt(String::from("Play"))
                    .set_screen_pos(Point::new(1, 3))
                    .set_event(ui::Event::Exit(PLAY_SEEDED)),
            ),
            Point::new(1, 3),
        );
        seed_scene.add_element(
            Box::new(
                basic_button
                    .clone()
                    .set_txt(String::from("Back"))
                    .set_screen_pos(Point::new(7, 3))
                    .set_event(ui::Event::ChangeScene(0)),
            ),
            Point::new(2, 3),
        );
        seed_scene.add_element(
            Box::new(ui::widgets::Outline::new('#'.grey(), seed_wid)),
            Point::new(999, 999),
        );

        seed_scene.move_cursor(Point::new(1, 2));
        menu_container.add_scene(seed_scene);

        // Death / win_screen.
        let mut end_scene = ui::Scene::new(Point::new(52, 21), 16, 5);

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
                    .set_txt(String::from("Save and Quit"))
                    .set_event(ui::Event::Exit(QUIT))
                    .set_screen_pos(Point::new(1, 3)),
            ),
            Point::new(1, 3),
        );
        end_scene.add_element(
            Box::new(ui::widgets::Outline::new('#'.grey(), 16)),
            Point::new(999, 999),
        );
        end_scene.move_cursor(Point::new(1, 1));

        menu_container.add_scene(end_scene);

        // Puzzle selection screen.
        let mut pzl_scene = ui::Scene::new(Point::new(51, 20), 18, 5)
            .with_scrolling(true);

        // Last seen difficulty during puzzle screen generation.
        let mut last_diff = -1;

        for (n, pzl) in pzls.iter().enumerate() {
            let pos = Point::new(1, n as i32 + 2);
            let mut screen_pos = pos + Point::new(0, last_diff);
            let strs = if let Some(s) = stars_earned.get(&pzls[n].id) { *s } else { 0 };
            let str1 = if strs >= 1 { '*' } else { ' ' };
            let str2 = if strs >= 2 { '*' } else { ' ' };

            // New difficulty block found
            let this_diff = pzl.diff as i32;
            if last_diff != this_diff {
                last_diff = this_diff;
                let clr = match this_diff {
                    0 => style::Color::Green,
                    1 => style::Color::Yellow,
                    2 => style::Color::Red,
                    3 => style::Color::DarkRed,
                    d => panic!("Unexpected difficulty '{d}'"),
                };
                
                pzl_scene.add_element(
                    Box::new(
                        basic_button
                            .clone()
                            .set_txt(format!("{}", pzl.diff))
                            .set_clr(clr)
                            .set_screen_pos(screen_pos)
                    ), pos + Point::new(500, 5));
                screen_pos.y += 1;
            }

            pzl_scene.add_element(
                Box::new(
                    basic_button
                        .clone()
                        .set_txt(format!("Puzzle {} {str1}{str2}", n+1))
                        .set_event(ui::Event::Exit(n as u32 + 100))
                        .set_screen_pos(screen_pos)
                ), pos);

            // Add a main menu button.
            if n == pzls.len() - 1 {
                pzl_scene.add_element(
                    Box::new(
                        basic_button
                            .clone()
                            .set_txt(String::from("Main Menu"))
                            .set_event(ui::Event::ChangeScene(0))
                            .set_screen_pos(screen_pos + Point::new(0, 2))
                    ), pos + Point::new(0, 1));
            }
        }
        pzl_scene.add_element(
            Box::new(ui::widgets::Outline::new('#'.grey(), 18)),
            Point::new(999, 999),
        );

        pzl_scene.move_cursor(Point::new(1, 2));

        menu_container.add_scene(pzl_scene);

        // Alternate end screen for puzzles.
        let mut puzzle_end = ui::Scene::new(Point::new(52, 18), 16, 7);
        let next = basic_button
                .clone()
                .set_txt(String::from("Next Puzzle"))
                .set_event(ui::Event::Exit(NEXT_PUZZLE));
        let retry = basic_button
                .clone()
                .set_txt(String::from("Retry"))
                .set_event(ui::Event::Exit(QUICK_RESET));

        puzzle_end.add_element(
            Box::new(
                next.clone().set_screen_pos(Point::new(1, 1))
            ),
            Point::new(1, 1),
        );
        puzzle_end.add_element(
            Box::new(
                retry.clone().set_screen_pos(Point::new(1, 2))
            ),
            Point::new(1, 2),
        );
        puzzle_end.add_element(
            Box::new(
                basic_button
                    .clone()
                    .set_txt(String::from("Puzzle Select"))
                    .set_event(ui::Event::Exit(PUZZLE_SELECT))
                    .set_screen_pos(Point::new(1, 3)),
            ),
            Point::new(1, 3),
        );
        puzzle_end.add_element(
            Box::new(
                basic_button
                    .clone()
                    .set_txt(String::from("Main Menu"))
                    .set_event(ui::Event::Exit(MAIN_MENU))
                    .set_screen_pos(Point::new(1, 4)),
            ),
            Point::new(1, 4),
        );
        puzzle_end.add_element(
            Box::new(
                basic_button
                    .clone()
                    .set_txt(String::from("Save and Quit"))
                    .set_event(ui::Event::Exit(QUIT))
                    .set_screen_pos(Point::new(1, 5)),
            ),
            Point::new(1, 5),
        );
        puzzle_end.add_element(
            Box::new(ui::widgets::Outline::new('#'.grey(), 16)),
            Point::new(999, 999),
        );

        // End screen if the player dies in a puzzle. Gives the option to retry at the top instead.
        let mut dead_puzzle_end = puzzle_end.clone();
        dead_puzzle_end.add_element(
            Box::new(
                retry.set_screen_pos(Point::new(1, 1))
            ),
            Point::new(1, 1)
        );
        dead_puzzle_end.add_element(
            Box::new(
                next.set_screen_pos(Point::new(1, 2))
            ),
            Point::new(1, 2)
        );

        puzzle_end.move_cursor(Point::new(1, 1));
        dead_puzzle_end.move_cursor(Point::new(1, 1));

        menu_container.add_scene(puzzle_end);
        menu_container.add_scene(dead_puzzle_end);

        if insta_puzzle {
            menu_container.change_scene(3);
            insta_puzzle = false;
        }

        if !quick_restart {
            match menu_container.run() {
                QUIT => break 'full,
                PLAY => (),
                PLAY_SEEDED => unsafe {
                    let txt = &menu_container.scenes[1]
                        .get_element(Point::new(1, 2))
                        .unwrap()
                        .get_text();
                    // Hash the string to get the seed if it is not in hex.
                    SEED = match u64::from_str_radix(txt, 16) {
                        Ok(val) => val,
                        Err(_) => u64::from_ne_bytes(md5::compute(txt).0[0..8].try_into().unwrap()),
                    };
                },
                // Puzzle selected.
                c if c >= 100 && c < 100 + pzl_count as u32 => {
                    unsafe { 
                        PUZZLE = Some(c as usize - 100);
                    }
                },
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
        main_wins.add_win(windowed::Window::new(PUZZLE_POS));

        // Seed the rng.
        floor_rng = rand::rngs::SmallRng::seed_from_u64(unsafe { SEED });

        // Map used through the game.
        let mut map: bn::Map<En> = unsafe {
            if let Some(idx) = PUZZLE {
                let pzl = &pzls[idx];
                PLAYER = pzl.pl_pos;
                ENEMIES_REMAINING = pzl.data.get_entities().count() - 1;
                pzl.data.clone()
            } else {
                let mut map = bn::Map::new(69, 69);

                // Generate the initial floor.
                gen_floor(
                    &mut map,
                    &mut floor_rng,
                    FLOORS_CLEARED,
                    &meta,
                    &templates,
                    &elites,
                );

                map
            }
        };

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
                            event::KeyCode::Char('n') => unsafe {
                                if CHEATS {
                                    NEXT_FLOOR = true;
                                    ENEMIES_REMAINING = 0;
                                    if FLOORS_CLEARED + 1 == KILL_SCREEN as u32 {
                                        break 'main;
                                    }
                                    ActionType::Wait
                                } else {
                                    continue;
                                }
                            },
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
                            // Reveal the map.
                            event::KeyCode::Char('R') => {
                                if CHEATS {
                                    let rev = *REVEALED.read().unwrap();
                                    *REVEALED.write().unwrap() = !rev;

                                    let mut write = LOG_MSGS.write().unwrap();
                                    write.push(LogMsg::new(format!(
                                        "{} {}s seeing all",
                                        templates::PLAYER_CHARACTER,
                                        if rev { "stop" } else { "start" }
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
                            }
                            // Kill everyone in the room.
                            event::KeyCode::Char('*') => unsafe {
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
                                    write.push(LogMsg::new(String::from("extended warranty of")));
                                    write.push(LogMsg::new(String::from("the enemies' vehicles")));
                                    ActionType::Wait
                                } else {
                                    continue;
                                }
                            },
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
                        if PUZZLE.is_some() {
                            break 'main;
                        }
                        FLOORS_CLEARED += 1;
                        if FLOORS_CLEARED == KILL_SCREEN as u32 {
                            break 'main;
                        }
                        NEXT_FLOOR = false;
                        gen_floor(
                            &mut map,
                            &mut floor_rng,
                            FLOORS_CLEARED,
                            &meta,
                            &templates,
                            &elites,
                        );
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
        let is_puzzle = unsafe { PUZZLE.is_some() };

        if !is_puzzle {
            // Real time taken.
            add_line(
                style::Color::White,
                &format!("Time Elapsed: {}:{:02}", time_taken / 60, time_taken % 60,),
                cur_win,
                main_wid,
            );
        }

        let turns = unsafe { GLOBAL_TIME };
        // In game time taken.
        let mut turn_msg = format!("Turns: {}", turns);
        
        if is_puzzle {
            unsafe { 
                let idx = *PUZZLE.as_ref().unwrap();
                let move_lim = pzls[idx].move_lim;
                let stars = if DEAD {
                    0
                } else {
                    turn_msg = format!("{turn_msg}/{move_lim}");
                    if turns > move_lim {
                        1
                    } else {
                        2
                    }
                };
                stars_earned.entry(pzls[idx].id).and_modify(|s| *s = std::cmp::max(*s, stars)).or_insert(stars);
                let msg = match stars {
                    0 => "0 stars...",
                    1 => "1 star.",
                    2 => "2 stars!",
                    _ => "hacks, apparently",
                };
                add_line(
                    style::Color::White,
                    &format!("You get {msg}"),
                    cur_win,
                    main_wid,
                );
            }
        }

        add_line(
            style::Color::White,
            &turn_msg,
            cur_win,
            main_wid,
        );

        if !is_puzzle {
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

            // Seed used.
            add_line(
                style::Color::White,
                &format!("Seed: {:X}", unsafe { SEED },),
                cur_win,
                main_wid,
            );
        }

        add_line(style::Color::White, "", cur_win, main_wid);

        cur_win.outline_with('#'.stylize());
        thread::sleep(time::Duration::from_millis(275));
        end_wins.refresh();
        print_win(&end_wins);

        menu_container.change_scene(if is_puzzle { 
            if unsafe { DEAD } {
                5
            } else {
                4
            }
        } else {
            2
        });
        match menu_container.run() {
            QUIT => break 'full,
            MAIN_MENU => (),
            QUICK_RESET => quick_restart = true,
            // This is necessary to ensure the screen is reloaded.
            PUZZLE_SELECT => insta_puzzle = true,
            NEXT_PUZZLE => unsafe {
                let cur_puz = *PUZZLE.as_ref().unwrap();
                if cur_puz != pzls.len() - 1 {
                    quick_restart = true;
                    PUZZLE = Some(cur_puz + 1);
                }
            }
            c if c >= 100 && c < 100 + pzl_count as u32 => {
                unsafe { 
                    PUZZLE = Some(c as usize - 100);
                    quick_restart = true;
                }
            },
            c => panic!("Unexpected code '{c}'"),
        }
    }

    // Write puzzle progress to file.
    puzzle_loader::pzl_save::write_pzl_save(stars_earned);

    // Put the terminal in a "normal" state in case the player actually wants to use it afterwards.
    terminal::disable_raw_mode();
    execute!(
        io::stdout(),
        terminal::Clear(terminal::ClearType::All),
        cursor::MoveTo(0, 0),
        cursor::Show,
    );

}

/// Clears all events currently in the queue.
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

    for sd in init_seed..init_seed + sds {
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
                let hell_ps = [Point::new(x - 1, y), Point::new(x, y - 1)];
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

                    if hell_ps.into_iter().all(|p| test(map.get_map(p))) {
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

        let init_seed = rand::rng().random_range(0..u64::MAX - sds);
        let found_fault = check_seeds(init_seed, sds);
        if found_fault {
            panic!();
        }
    }
}
