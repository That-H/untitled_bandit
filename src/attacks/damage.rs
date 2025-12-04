//! Contains basic damage objects.

/// Some basic effect of an attack.
#[derive(Clone, Copy, Debug)]
pub enum DmgType {
    /// Heal the target by this amount.
    Heal(u32),
    /// Damage the target by this amount.
    Dmg(u32),
}

impl DmgType {
    /// Creates a new damage type instance using dmg; a negative dmg
    /// is interpreted as a heal.
    pub fn new(dmg: i32) -> Self {
        let new_dmg = dmg.unsigned_abs();
        if dmg < 0 {
            Self::Heal(new_dmg)
        } else {
            Self::Dmg(new_dmg)
        }
    }
}

/// An instance of damage against a target.
#[derive(Clone, Copy, Debug)]
pub struct DmgInst {
    /// Basic effect.
    pub dmg: DmgType,
    /// Chance of hitting from 0 to 1.
    pub acc: f64,
}

impl DmgInst {
    /// Create a damage instance with the given accuracy and damage.
    pub fn dmg(dmg: u32, acc: f64) -> Self {
        Self {
            dmg: DmgType::Dmg(dmg),
            acc,
        }
    }

    /// Create a damge instance that heals with 1.0 accuracy.
    pub fn heal(heal: u32) -> Self {
        Self {
            dmg: DmgType::Heal(heal),
            acc: 1.0,
        }
    }

    /// Create a damge instance that heals with acc accuracy.
    pub fn heal_with(heal: u32, acc: f64) -> Self {
        Self {
            dmg: DmgType::Heal(heal),
            acc,
        }
    }

    /// Returns the amount of damage this instance deals.
    pub fn total_dmg(&self) -> i32 {
        match self.dmg {
            DmgType::Heal(h) => -(h as i32),
            DmgType::Dmg(d) => d as i32,
        }
    }
}
