#![allow(static_mut_refs)]

use crossterm::style::{self, Stylize};
use rect::Rect;
use std::fmt;
use std::ops::{self, Deref};

pub const DELAY: u64 = 30;
pub const VFX_DELAY: u64 = 200;
pub const MAP_OFFSET: usize = 0;
/// Floor where the game ends.
pub const KILL_SCREEN: usize = 4;
pub const DIR_CHARS: [char; KILL_SCREEN] = ['v', '<', '^', '>'];
pub const DOOR_CHAR: char = '/';
pub const DOOR_CLRS: [style::Color; KILL_SCREEN] = [
    style::Color::White,
    style::Color::DarkGrey,
    style::Color::Rgb {
        r: 255,
        g: 165,
        b: 0,
    },
    style::Color::DarkYellow,
];
pub const WALL_CLRS: [style::Color; KILL_SCREEN] = [
    style::Color::DarkGrey,
    style::Color::White,
    style::Color::DarkMagenta,
    style::Color::DarkRed,
];
pub const ICE_CHAR: char = '*';
pub const ICE_CLR: style::Color = style::Color::Cyan;

pub use bandit as bn;
pub use bn::Point;
use bn::Tile as Ti;

use crate::entity::FLOORS_CLEARED;

pub mod attacks;

pub mod map_gen;

pub mod entity;

pub mod templates;

pub mod ui;

/// Returns the colour of doors on the current floor.
pub fn get_door_clr() -> style::Color {
    DOOR_CLRS[unsafe { FLOORS_CLEARED as usize }]
}

/// A single tile in a map.
pub struct Tile {
    /// Whether there is anything there or not.
    pub empt: bool,
    /// Whether or not passage is allowed through this tile.
    pub blocking: bool,
    /// Whether the tile has been seen before.
    pub revealed: bool,
    /// Character used to represent this tile.
    pub ch: Option<StyleCh>,
    /// The rooms the tile connects.
    pub door: Option<(Rect, Rect)>,
    /// Key type required to change this tile to not be blocking.
    pub locked: Option<u32>,
    /// Whether the tile engages sliding.
    pub slippery: bool,
    /// Something that occurs when an entity steps on this tile. The arguments are the position
    /// of the tile and a commands instance with which to actuate effects.
    pub step_effect: Option<Box<StepEffect>>,
}

type StepEffect = dyn Fn(Point, &bn::Map<entity::En>) -> Vec<bn::Cmd<entity::En>>;

impl Tile {
    /// Create a new revealed empty tile.
    pub fn new_empty() -> Self {
        Self {
            revealed: true,
            ..Self::default()
        }
    }

    /// If the tile is locked and the corresponding key has been collected, unlocks the door.
    pub fn unlock(&mut self) {
        if self.unlockable() {
            let lck_val = self.locked.take().unwrap() as usize;
            self.blocking = false;
            self.ch = Some(DOOR_CHAR.with(get_door_clr()));
            unsafe { crate::entity::KEYS_COLLECTED[lck_val] -= 1 }
            entity::LOG_MSGS
                .write()
                .unwrap()
                .push(format!("{} unlocks door", templates::PLAYER_CHARACTER).into());
        }
    }

    /// Returns true if the corresponding key to the door has been collected.
    pub fn unlockable(&self) -> bool {
        if let Some(k) = self.locked
            && unsafe { crate::entity::KEYS_COLLECTED[k as usize] > 0 }
        {
            true
        } else {
            false
        }
    }
}

impl Default for Tile {
    fn default() -> Self {
        Self {
            empt: true,
            blocking: false,
            revealed: false,
            ch: None,
            door: None,
            slippery: false,
            step_effect: None,
            locked: None,
        }
    }
}

impl fmt::Display for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.repr())
    }
}

impl bn::Tile for Tile {
    type Repr = StyleCh;

    fn repr(&self) -> Self::Repr {
        let flrs = unsafe { crate::entity::FLOORS_CLEARED as usize };
        if !self.revealed {
            ' '.stylize()
        } else if let Some(c) = self.ch {
            c
        } else if self.blocking {
            '#'.with(WALL_CLRS[flrs])
        } else if !self.empt {
            '.'.with(WALL_CLRS[flrs])
        } else {
            ' '.stylize()
        }
    }
}

type StyleCh = style::StyledContent<char>;

/// A singular frame of animation in a visual effect.
#[derive(Clone, Debug)]
pub enum Frame {
    /// Change nothing.
    Transparent,
    /// Replace with the given styled character.
    Opaque(StyleCh),
    /// Set the colour that the character is on.
    ReplaceFloor(style::Color),
    /// Compute the new text in some other way.
    Other(Box<fn(&StyleCh) -> StyleCh>),
}

impl Frame {
    /// Turn the original text at this position into something new.
    pub fn map(&self, txt: &StyleCh) -> StyleCh {
        match self {
            Self::Transparent => *txt,
            Self::Opaque(ch) => *ch,
            Self::ReplaceFloor(clr) => txt.on(*clr),
            Self::Other(cl) => cl(txt),
        }
    }
}

/// A visual effect in the grid.
#[derive(Clone, Debug)]
pub struct Vfx {
    frames: Vec<Frame>,
    cur_idx: usize,
}

impl Vfx {
    /// Create a new instance with the given frames.
    pub fn new(frames: Vec<Frame>) -> Self {
        Self { frames, cur_idx: 0 }
    }

    /// Create a new instance with frames copies of the given character
    /// as opaque frames.
    pub fn new_opaque(ch: StyleCh, frames: usize) -> Self {
        Self {
            frames: vec![Frame::Opaque(ch); frames],
            cur_idx: 0,
        }
    }

    /// Create a new instance with frames copies of the given character
    /// coloured using clr as opaque frames.
    pub fn opaque_with_clr(ch: char, clr: style::Color, frames: usize) -> Self {
        Self {
            frames: vec![Frame::Opaque(ch.with(clr)); frames],
            cur_idx: 0,
        }
    }
}

impl bn::Vfx for Vfx {
    type Txt = StyleCh;

    fn update(&mut self) -> bool {
        self.cur_idx += 1;
        self.cur_idx == self.frames.len()
    }

    fn modify_txt(&self, txt: &Self::Txt) -> Self::Txt {
        self.frames[self.cur_idx].map(txt)
    }
}

/// Stores a value and ensures it does not exceed a maximum.
#[derive(Clone)]
pub struct Datum<T: Clone + PartialOrd> {
    /// Maximum value of the datum.
    pub max: T,
    cur: Option<T>,
}

impl<T: Clone + PartialOrd> Datum<T> {
    /// Create a new Datum with the given max value. The current value will also
    /// be set to this value.
    pub fn new(max: T) -> Self {
        Self { cur: None, max }
    }

    /// Set the current value to the given one if it is less than max.
    pub fn set_to(&mut self, new_val: T) {
        self.cur = if new_val > self.max {
            None
        } else {
            Some(new_val)
        }
    }

    /// Reset to the max value.
    pub fn reset(&mut self) {
        self.cur = None;
    }

    /// Return a reference to the current value stored.
    pub fn value(&self) -> &T {
        self.deref()
    }
}

impl<T: Clone + PartialOrd> PartialEq<T> for Datum<T> {
    fn eq(&self, other: &T) -> bool {
        (**self) == *other
    }
}

impl<T: Clone + PartialOrd> ops::AddAssign<T> for Datum<T>
where
    for<'a> &'a T: ops::Add<T, Output = T>,
{
    fn add_assign(&mut self, other: T) {
        self.set_to((*self).deref() + other)
    }
}

impl<T: Clone + PartialOrd> ops::SubAssign<T> for Datum<T>
where
    for<'a> &'a T: ops::Sub<T, Output = T>,
{
    fn sub_assign(&mut self, other: T) {
        self.set_to((*self).deref() - other)
    }
}

impl<T: Clone + PartialOrd> ops::MulAssign<T> for Datum<T>
where
    for<'a> &'a T: ops::Mul<T, Output = T>,
{
    fn mul_assign(&mut self, other: T) {
        self.set_to((*self).deref() * other)
    }
}

impl<T: Clone + PartialOrd> ops::DivAssign<T> for Datum<T>
where
    for<'a> &'a T: ops::Div<T, Output = T>,
{
    fn div_assign(&mut self, other: T) {
        self.set_to((*self).deref() / other)
    }
}

impl<T: Clone + PartialOrd> ops::Deref for Datum<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self.cur.as_ref() {
            Some(c) => c,
            None => &self.max,
        }
    }
}

/// Some action the player can take.
#[derive(Clone, Debug)]
pub enum ActionType {
    /// Try to move with the given displacement.
    TryMove(Point),
    /// Use a melee attack against the player if possible. If multiple are possible,
    /// the first one found will be used.
    TryMelee,
    /// Use the melee attack with the given direction and index, regardless of whether it would do anything.
    ForceMelee(Point, usize),
    /// Use the ranged attack at the given index.
    Fire(usize),
    /// Pathfind towards the player.
    Pathfind,
    /// Do nothing.
    Wait,
    /// Does both actions, regardless of success.
    Multi(Box<ActionType>, Box<ActionType>),
    /// Does the first action, and if it fails, does the second one.
    Chain(Box<ActionType>, Box<ActionType>),
    /// Does the action at the first idx given if the predicate evaluates to true,
    /// otherwise does the action at the other idx given.
    /// As arguments, the predicate takes the current map, the entity currently acting,
    /// and the position of the entity in the map.
    CondBranch(
        usize,
        usize,
        Box<fn(&bn::Map<entity::En>, &entity::En, Point) -> bool>,
    ),
    /// Uses the provided function to generate [commands](bn::Cmd) directly, given the environment.
    Arbitrary(Box<fn(&bn::Map<entity::En>, &entity::En, Point) -> Vec<bn::Cmd<entity::En>>>),
}
