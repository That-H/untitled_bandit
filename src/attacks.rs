//! Contains objects used in attacking.
use crate::{Point, Vfx};
use crossterm::style;
use std::collections::HashMap;

pub const THICC_FOUR_POS_ATK: [char; 4] = ['═', '║', '═', '║'];
pub const FOUR_POS_ATK: [char; 4] = ['-', '|', '-', '|'];
pub const EIGHT_POS_ATK: [char; 8] = ['/', '-', '\\', '|', '|', '\\', '-', '/'];

mod damage;
pub use damage::*;

use crate::bn;

type OtherEf = fn(Point, Point, &bn::Map<super::entity::En>) -> Vec<bn::Cmd<super::entity::En>>;

/// Some effect that can occur as a result of an attack.
#[derive(Clone, Debug)]
pub enum Effect {
	/// Apply the damage instance to the entity.
	DoDmg(DmgInst),
	/// Do something else. Arguments are (in that order):
	/// - Where you attack from
	/// - Where you attack
	/// - Map in which the attack takes place
	Other(OtherEf)
}

#[derive(Clone, Debug)]
pub struct MeleeAtk {
    /// Damage dealt.
    pub effects: Vec<Effect>,
    /// Where it affects relative to some position.
    pub place: Vec<Point>,
    /// Each effect that will be displayed relative to some position. Will be overwritten by
    /// miss_fx where applicable.
    pub fx: Vec<(Point, Vfx)>,
    /// Effect that will be displayed each place the attack misses.
    pub miss_fx: Vfx,
}

impl MeleeAtk {
    /// Create a new melee attack.
    pub fn new(effects: Vec<Effect>, place: Vec<Point>, fx: Vec<(Point, Vfx)>, miss_fx: Vfx) -> Self {
        Self {
            effects,
            place,
            fx,
            miss_fx,
        }
    }

    /// Create amount attacks with the given vfx parameters. An amount of four leads to
    /// the four orthogonally adjacent positions being attackable, and an amount of eight
    /// also includes diagonally adjacent positions.
    pub fn bulk_new<'a, const N: usize>(
        effects: Vec<Effect>,
        clr: style::Color,
        frames: usize,
        miss_fx: Vfx,
        chars: impl IntoIterator<Item = &'a char>,
    ) -> Vec<Self> {
        let mut bulk = Vec::new();
        let get_atk = |pos: Point, ch: char| {
            MeleeAtk::new(
                effects.clone(),
                vec![pos],
                vec![(pos, Vfx::opaque_with_clr(ch, clr, frames))],
                miss_fx.clone(),
            )
        };

        if N == 4 {
            for (pos, ch) in Point::ORIGIN
                .get_all_adjacent()
                .into_iter()
                .zip(chars.into_iter())
            {
                bulk.push(get_atk(pos, *ch));
            }
        } else if N == 8 {
            for (pos, ch) in Point::ORIGIN
                .get_all_adjacent_diagonal()
                .into_iter()
                .zip(chars.into_iter())
            {
                bulk.push(get_atk(pos, *ch));
            }
        } else {
            panic!("Expected to create 4 or 8 attacks, but got {N}");
        }

        bulk
    }

    /// Returns whether the attack can hit the target position from the given position.
    pub fn hits(&self, from: Point, target: Point) -> bool {
        self.place.iter().any(|p| (*p + from) == target)
    }
}

type CalcLineFx = fn(bool, Vec<Point>) -> Vec<(Point, Vfx)>;

#[derive(Clone)]
pub struct RangedAtk {
    /// Damage dealt.
    pub effects: Vec<Effect>,
    /// Maximum distance from which the attack can be used.
    pub range: u32,
    /// Each effect that will be displayed relative to some position.
    pub fx: Vec<(Point, Vfx)>,
    /// Closure used to convert a given line into some visual effects.
    pub line_fx: Box<CalcLineFx>,
}

impl RangedAtk {
    /// Create a new ranged attack.
    pub fn new(
        effects: Vec<Effect>,
        range: u32,
        fx: Vec<(Point, Vfx)>,
        line_fx: Box<CalcLineFx>,
    ) -> Self {
        Self {
            effects,
            range,
            fx,
            line_fx,
        }
    }
}

/// Stores melee and ranged attacks. Associates the melee attacks with a direction.
#[derive(Clone)]
pub struct AtkPat {
    /// Melee attacks.
    pub melee_atks: HashMap<Point, Vec<MeleeAtk>>,
    /// Ranged attacks.
    pub ranged_atks: Vec<RangedAtk>,
}

impl AtkPat {
    /// Return an empty attack pattern.
    pub fn empty() -> Self {
        Self {
            melee_atks: HashMap::new(),
            ranged_atks: Vec::new(),
        }
    }

    /// Create an attack pattern from the given melee attacks, which are assumed to have
    /// been created by [MeleeAtk::bulk_new]. Will panic if there are not 4 or 8 attacks in the
    /// vector.
    pub fn from_atks(atks: Vec<MeleeAtk>) -> Self {
        let len = atks.len();
        let adj;

        if len == 4 {
            adj = Point::ORIGIN.get_all_adjacent();
        } else if len == 8 {
            adj = Point::ORIGIN.get_all_adjacent_diagonal();
        } else {
            panic!("Expected 4 or 8 attacks but got {len}");
        }

        Self {
            melee_atks: HashMap::from_iter(adj.into_iter().zip(atks.into_iter().map(|a| vec![a]))),
            ranged_atks: Vec::new(),
        }
    }

    /// Finds all the positions from which one could melee attack the given point.
    pub fn find_attack_positions(&self, to: Point) -> Vec<Point> {
        let mut possible = Vec::new();

        for atks in self.melee_atks.values() {
            for atk in atks.iter() {
                for pos in atk.place.iter() {
                    possible.push(to - *pos);
                }
            }
        }

        possible
    }

    // Finds all positions affected by melee attacks in the given direction from the given position.
    pub fn attacked_from(&self, from: Point, dir: Point) -> Vec<Point> {
        let mut possible = Vec::new();

        if let Some(atks) = self.melee_atks.get(&dir) {
            for atk in atks.iter() {
                for disp in atk.place.iter() {
                    possible.push(from + *disp);
                }
            }
        }

        possible
    }

    // Returns the direction needed to hit the target from the given start position,
    // if there is one, as well as the index of the first attack in that direction
    // that hits the target.
    pub fn melee_hit_from(&self, from: Point, target: Point) -> Option<(Point, usize)> {
        for (dir, atks) in self.melee_atks.iter() {
            for (n, atk) in atks.iter().enumerate() {
                if atk.hits(from, target) {
                    return Some((*dir, n));
                }
            }
        }

        None
    }
}
