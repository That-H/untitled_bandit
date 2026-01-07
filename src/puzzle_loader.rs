//! Loads puzzle files and turns them into playable maps.

use crate::*;
use std::str::FromStr;
use std::{
    fs,
    fmt,
    io::{self, BufRead},
};

pub mod ts;

/// Represents the subjective difficulty of a puzzle.
#[derive(Debug, Clone, Copy)]
pub enum Difficulty {
    Beginner,
    Intermediate,
    Advanced,
}

impl fmt::Display for Difficulty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match *self {
            Self::Beginner => "Beginner",
            Self::Intermediate => "Intermediate",
            Self::Advanced => "Advanced",
        })
    }
}

impl FromStr for Difficulty {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "B" => Self::Beginner,
            "I" => Self::Intermediate,
            "A" => Self::Advanced,
            _ => return Err(()),
        })
    }
}

/// Partially populated puzzle.
#[derive(Default)]
struct PuzzleBuilder {
    /// Map of the puzzle.
    data: Option<bn::Map<entity::En>>,
    /// Location of the player in the map.
    pl_pos: Option<Point>,
    /// Difficulty of the puzzle.
    diff: Option<Difficulty>,
    /// Maximum number of moves allowed to complete the puzzle and get two stars.
    move_lim: Option<u32>,
}

impl PuzzleBuilder {
    /// Create an empty puzzle builder.
    fn new() -> Self {
        Self::default()
    }

    /// Check it contains all necessary data for a puzzle.
    fn check(&self) -> bool {
        self.data.is_some()
            && self.pl_pos.is_some()
            && self.diff.is_some()
            && self.move_lim.is_some()
    }
}

/// Contains all necessary information for a puzzle.
pub struct Puzzle {
    /// Map of the puzzle.
    pub data: bn::Map<entity::En>,
    /// Location of the player in the map.
    pub pl_pos: Point,
    /// Difficulty of the puzzle.
    pub diff: Difficulty,
    /// Maximum number of moves allowed to complete the puzzle and get two stars.
    pub move_lim: u32,
}

impl Puzzle {
    /// Create an empty puzzle.
    pub fn new(diff: Difficulty, move_lim: u32) -> Self {
        Self {
            data: bn::Map::new(50, 50),
            pl_pos: Point::ORIGIN,
            diff,
            move_lim,
        }
    }
}

impl TryFrom<PuzzleBuilder> for Puzzle {
    type Error = ();

    fn try_from(value: PuzzleBuilder) -> Result<Self, Self::Error> {
        if value.check() {
            Ok(Puzzle {
                data: value.data.unwrap(),
                pl_pos: value.pl_pos.unwrap(),
                diff: value.diff.unwrap(),
                move_lim: value.move_lim.unwrap(),
            })
        } else {
            Err(())
        }
    }
}

/// Turns a string into a map using a tile set.
fn create_map(data: &str, tile_set: &ts::TileSet, default_tile: &Tile) -> PuzzleBuilder {
    let mut map = bn::Map::new(69, 69);
    let mut builder = PuzzleBuilder::new();

    for (y, ln) in data.lines().rev().enumerate() {
        for (x, ch) in ln.chars().enumerate() {
            let pos = Point::new(x as i32, y as i32);

            if let Some(obj) = tile_set.map(ch) {
                match obj {
                    ts::BanditObj::Tile(t) => map.insert_tile(t.clone(), pos),
                    ts::BanditObj::En(en) => {
                        if en.is_player {
                            builder.pl_pos.replace(pos);
                        }
                        map.insert_entity(en.clone(), pos);
                        map.insert_tile(default_tile.clone(), pos);
                    }
                }
            }
        }
    }

    builder.data.replace(map);
    builder
}

/// Uses the given tileset to turn a string into a puzzle. Unknown characters will be ignored.
pub fn load_pzl(
    data: &str,
    default_tile: &Tile,
    tile_set: &ts::TileSet,
    diff: Difficulty,
    move_lim: u32,
) -> Puzzle {
    let mut pzl = Puzzle::new(diff, move_lim);

    let PuzzleBuilder {
        data,
        pl_pos,
        diff: _diff,
        move_lim: _move_lim,
    } = create_map(data, tile_set, default_tile);

    pzl.data = data.unwrap();
    pzl.pl_pos = pl_pos.unwrap();
    pzl
}

/// Takes a file and loads all puzzles from it, assuming the file is stored in the correct format.
pub fn load_pzls<P: AsRef<std::path::Path>>(
    fname: P,
    default_tile: &Tile,
    tile_set: &ts::TileSet,
) -> Result<Vec<Puzzle>, ()> {
    let mut pzls = Vec::new();
    let file = fs::File::open(fname).unwrap();
    let mut state = 0;
    let mut data = String::new();
    let mut builder = PuzzleBuilder::new();

    for line in io::BufReader::new(file).lines().map_while(Result::ok) {
        match state {
            // Read difficulty and move limit.
            0 => {
                for (n, val) in line.split(' ').enumerate() {
                    match n {
                        0 => {
                            builder.move_lim.replace(val.parse().map_err(|_e| ())?);
                        }
                        1 => {
                            builder.diff.replace(val.parse().map_err(|_e| ())?);
                        }
                        _ => break,
                    }
                }

                state = 1;
            }
            // Add lines to data until an empty line is found, then load the puzzle.
            1 => {
                if line.is_empty() {
                    let pzl = load_pzl(
                        &data,
                        default_tile,
                        tile_set,
                        builder.diff.unwrap(),
                        builder.move_lim.unwrap(),
                    );
                    pzls.push(pzl);
                    data = String::new();
                    builder = PuzzleBuilder::new();
                    state = 0;
                } else {
                    data.push_str(&line);
                    data.push('\n');
                }
            }
            _ => unreachable!(),
        }
    }

    Ok(pzls)
}
