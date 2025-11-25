use crossterm::style::{self, Stylize};
use rect::Rect;
use std::fmt;
use std::ops::{self, Deref};

pub const DELAY: u64 = 30;
pub const VFX_DELAY: u64 = 200;
pub const MAP_OFFSET: usize = 0;
pub const DIR_CHARS: [char; 4] = ['v', '<', '^', '>'];

pub use bandit as bn;
pub use bn::Point;
use bn::Tile as Ti;

pub mod attacks;

pub mod map_gen;

pub mod entity;

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
        if !self.revealed {
            ' '.stylize()
        } else if let Some(c) = self.ch {
            c
        } else if self.blocking {
            '#'.dark_grey()
        } else if !self.empt {
            '.'.dark_grey()
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
#[derive(Clone, Copy, Debug)]
pub enum ActionType {
    /// Try to move.
    TryMove,
    /// Use the ranged attack at the given index.
    Fire(usize),
    /// Do nothing.
    Wait,
}
