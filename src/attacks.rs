//! Contains objects used in attacking.
use crate::{Point, Vfx};
use crossterm::style;
use std::collections::HashMap;

const FOUR_POS_ATK: [char; 4] = ['-', '|', '-', '|'];
const EIGHT_POS_ATK: [char; 8] = ['/', '-', '\\', '|', '|', '\\', '-', '/'];

mod damage;
pub use damage::*;

#[derive(Clone, Debug)]
pub struct MeleeAtk {
    /// Damage dealt.
    pub effect: DmgInst,
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
    pub fn new(effect: DmgInst, place: Vec<Point>, fx: Vec<(Point, Vfx)>, miss_fx: Vfx) -> Self {
        Self {
            effect,
            place,
            fx,
            miss_fx,
        }
    }

    /// Create amount attacks with the given vfx parameters. An amount of four leads to
    /// the four orthogonally adjacent positions being attackable, and an amount of eight
    /// also includes diagonally adjacent positions. Each character used is one of -, |,
    /// / or \\ to represent the direction used.
    pub fn bulk_new(
        effect: DmgInst,
        clr: style::Color,
        frames: usize,
        miss_fx: Vfx,
        amount: u8,
    ) -> Vec<Self> {
        let mut bulk = Vec::new();
        let get_atk = |pos: Point, ch: char| {
            MeleeAtk::new(
                effect,
                vec![pos],
                vec![(pos, Vfx::opaque_with_clr(ch, clr, frames))],
                miss_fx.clone(),
            )
        };

        if amount == 4 {
            for (pos, ch) in Point::ORIGIN
                .get_all_adjacent()
                .into_iter()
                .zip(FOUR_POS_ATK.iter())
            {
                bulk.push(get_atk(pos, *ch));
            }
        } else if amount == 8 {
            for (pos, ch) in Point::ORIGIN
                .get_all_adjacent_diagonal()
                .into_iter()
                .zip(EIGHT_POS_ATK.iter())
            {
                bulk.push(get_atk(pos, *ch));
            }
        } else {
            panic!("Expected to create 4 or 8 attacks, but got {amount}");
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
    pub effect: DmgInst,
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
        effect: DmgInst,
        range: u32,
        fx: Vec<(Point, Vfx)>,
        line_fx: Box<CalcLineFx>,
    ) -> Self {
        Self {
            effect,
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
