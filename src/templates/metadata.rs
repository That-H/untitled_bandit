use std::collections::HashMap;
use crate::{get_assets_path, puzzle_loader::read_lines};

/// Name of the file containing the description of each enemy.
pub const DESC_FILE: &str = "desc.txt";

/// Metadata about templates.
pub struct TempMeta {
    /// Cost to spawn the enemy in a room.
    pub cost: u32,
    /// Range of floors it can spawn in.
    pub floor_rang: std::ops::RangeInclusive<u32>,
    /// Maximum amount of this entity that can spawn in a room.
    pub max: u32,
}

/// Return a mapping of the enemies' character representations to their descriptions.
pub fn get_descs() -> HashMap<char, String> {
    let mut state = 0;
    let mut map = HashMap::new();
    let mut desc = String::new();
    let mut ch = None;

    for ln in read_lines(get_assets_path().join(DESC_FILE)).unwrap().map_while(Result::ok) {
        if state == 0 {
            ch = Some(ln.chars().next().expect("Incorrect description formatting"));
            state = 1;
        } else if state == 1 {
            if ln.is_empty() {
                map.insert(ch.take().unwrap(), desc);
                desc = String::new();
                state = 0;
            } else {
                desc.push_str(&ln);
            }
        };
    }

    map
}

/// Returns metadata about templates.
pub fn get_metadata() -> HashMap<char, TempMeta> {
    HashMap::from([
        (
            'e',
            TempMeta {
                cost: 12,
                floor_rang: 0..=1,
                max: 3,
            },
        ),
        (
            'h',
            TempMeta {
                cost: 22,
                floor_rang: 0..=1,
                max: 2,
            },
        ),
        (
            'l',
            TempMeta {
                cost: 60,
                floor_rang: 3..=3,
                max: 1,
            },
        ),
        (
            'k',
            TempMeta {
                cost: 37,
                floor_rang: 1..=2,
                max: 2,
            },
        ),
        (
            'b',
            TempMeta {
                cost: 48,
                floor_rang: 3..=3,
                max: 1,
            },
        ),
        (
            'r',
            TempMeta {
                cost: 27,
                floor_rang: 1..=2,
                max: 3,
            },
        ),
        (
            'w',
            TempMeta {
                cost: 31,
                floor_rang: 1..=2,
                max: 2,
            },
        ),
        (
            'o',
            TempMeta {
                cost: 15,
                floor_rang: 0..=1,
                max: 3,
            },
        ),
        (
            'v',
            TempMeta {
                cost: 45,
                floor_rang: 2..=3,
                max: 2,
            },
        ),
        (
            'g',
            TempMeta {
                cost: 35,
                floor_rang: 2..=3,
                max: 2,
            },
        ),
        (
            'q',
            TempMeta {
                cost: 70,
                floor_rang: 3..=3,
                max: 1,
            },
        ),
        (
            'O',
            TempMeta {
                cost: 50,
                floor_rang: 1..=1,
                max: 1,
            },
        ),
        (
            'B',
            TempMeta {
                cost: 50,
                floor_rang: 2..=2,
                max: 1,
            },
        ),
        (
            'Q',
            TempMeta {
                cost: 50,
                floor_rang: 1..=1,
                max: 1,
            },
        ),
        (
            'E',
            TempMeta {
                cost: 50,
                floor_rang: 0..=0,
                max: 1,
            },
        ),
        (
            'R',
            TempMeta {
                cost: 50,
                floor_rang: 2..=2,
                max: 1,
            },
        ),
        (
            'Ω',
            TempMeta {
                cost: 50,
                floor_rang: 3..=3,
                max: 1,
            },
        ),
    ])
}
