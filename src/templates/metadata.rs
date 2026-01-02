use std::collections::HashMap;

/// Metadata about templates.
pub struct TempMeta {
    /// Cost to spawn the enemy in a room.
    pub cost: u32,
    /// Range of floors it can spawn in.
    pub floor_rang: std::ops::RangeInclusive<u32>,
    /// Maximum amount of this entity that can spawn in a room.
    pub max: u32,
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
            'E',
            TempMeta {
                cost: 50,
                floor_rang: 0..=0,
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
