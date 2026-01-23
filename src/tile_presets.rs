//! Contains some functions for generating commonly used tile types.

use super::*;
use crate::entity::*;

/// The four arrows.
pub const ARROWS: [char; 4] = ['↓', '←', '↑', '→'];
/// This does look like a key when printed.
pub const KEY: char = '⚷';
/// Colour of the exit on each floor.
pub const EXIT_CLRS: [style::Color; 4] = KEY_CLRS;
pub const LOCKED_DOOR: char = '╬';
type StepEffect = dyn StepEffectFn;

/// Return a conveyor tile pushing entities that step on it in the given direction.
pub fn create_conveyor(disp: Point, revealed: bool) -> Tile {
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
        door: false,
        step_effect,
        locked: None,
    }
}

/// Return a tile transporting the player to the given floor.
pub fn get_exit(revealed: bool, floor_num: usize) -> Tile {
    Tile {
        ch: Some('>'.with(EXIT_CLRS[floor_num % 4])),
        blocking: false,
        empt: false,
        revealed,
        door: false,
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

/// Return a tile that provides the player with a key.
pub fn get_key(revealed: bool, key_id: u32) -> Tile {
    Tile {
        ch: Some(KEY.with(KEY_CLRS[key_id as usize % 4])),
        blocking: false,
        empt: false,
        revealed,
        door: false,
        slippery: false,
        step_effect: Some(Box::new(move |pos, _| {
            unsafe { KEYS_COLLECTED[key_id as usize % KEY_CLRS.len()] += 1 }
            LOG_MSGS
                .write()
                .unwrap()
                .push(format!("{} gains key", templates::PLAYER_CHARACTER).into());
            vec![bn::Cmd::new_on(pos).modify_tile(Box::new(|t: &mut Tile| {
                t.step_effect = None;
                t.ch = Some('.'.with(WALL_CLRS[unsafe { FLOORS_CLEARED as usize }]));
            }))]
        })),
        locked: None,
    }
}

/// Return a tile that is locked and requires a key of the correct id.
pub fn get_locked_door(revealed: bool, key_id: u32) -> Tile {
    Tile {
        ch: Some(LOCKED_DOOR.with(KEY_CLRS[key_id as usize % 4])),
        blocking: true,
        empt: false,
        revealed,
        door: true,
        slippery: false,
        step_effect: None,
        locked: Some(key_id),
    }
}

