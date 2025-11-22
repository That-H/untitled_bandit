//! Contains all things to do with entities.

use super::*;
use crate::bn;
use attacks::*;
use bn::Entity;

/// Direction the player should go in.
pub static mut DIR: Point = Point::new(0, 0);
/// Type of action the player will perform.
pub static mut ACTION: ActionType = ActionType::Wait;
/// Current position of player.
pub static mut PLAYER: Point = Point::new(0, 0);
/// Are you dead yet?
pub static mut DEAD: bool = false;
/// Number of enemies remaining in the current room.
pub static mut ENEMIES_REMAINING: usize = 0;
/// Number of actions taken by the player.
pub static mut GLOBAL_TIME: u32 = 0;

/// Describes the way in which an entity differs from a normal entity.
#[derive(Clone, Copy)]
pub enum Special {
    /// Locked door.
    WallSentry,
    /// Anything that isn't special.
    Not,
}

/// A template for creating an entity from.
#[derive(Clone)]
pub struct EntityTemplate {
    pub max_hp: u32,
    pub delay: u8,
    pub movement: Vec<Point>,
    pub ch: style::StyledContent<char>,
    pub atks: AtkPat,
}

#[derive(Clone)]
pub struct En {
    /// Stores current and maximum hp of the entity.
    pub hp: Datum<u32>,
    /// Stores current speed of the entity.
    pub delay: u8,
    /// Is this entity the player?
    pub is_player: bool,
    /// How it is special.
    pub special: Special,
    /// Character representation of this entity.
    pub ch: style::StyledContent<char>,
    /// Each way the entity can attack its enemies.
    pub atks: AtkPat,
    /// Relative cells to which the entity could potentially move.
    pub movement: Vec<Point>,
    /// Is the entity visible and able to do stuff?
    pub dormant: bool,
    /// True if the entity acted this turn.
    acted: bool,
    /// Contains a value if the entity is forced to move in a specific direction.
    pub vel: Option<Point>,
}

impl En {
    /// Creates an entity with the provided data.
    pub fn new(
        max_hp: u32,
        is_player: bool,
        delay: u8,
        ch: style::StyledContent<char>,
        special: Special,
        movement: Vec<Point>,
        atks: AtkPat,
        dormant: bool,
    ) -> Self {
        Self {
            hp: Datum::new(max_hp),
            delay,
            is_player,
            special,
            ch,
            movement,
            atks,
            dormant,
            acted: false,
            vel: None,
        }
    }

    /// Creates an entity from a template and two entity specific data points.
    pub fn from_template(template: &EntityTemplate, is_player: bool, dormant: bool) -> Self {
        let EntityTemplate {
            max_hp,
            delay,
            movement,
            ch,
            atks,
        } = template.clone();

        Self::new(
            max_hp,
            is_player,
            delay,
            ch,
            Special::Not,
            movement,
            atks,
            dormant,
        )
    }

    /// Applies the given given damage instance to this entity. Returns whether
    /// or not it is still alive.
    pub fn apply_dmg(&mut self, dmg: DmgInst) -> bool {
        match dmg.dmg {
            DmgType::Heal(h) => {
                self.hp += h;
                false
            }
            DmgType::Dmg(d) => {
                if d > *self.hp {
                    self.hp.set_to(0);
                    true
                } else {
                    self.hp -= d;
                    false
                }
            }
        }
    }

    /// Returns true if this entity is medically dead (has 0 hp).
    pub fn is_dead(&self) -> bool {
        self.hp == 0
    }
}

impl fmt::Display for En {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.repr())
    }
}

impl bn::Entity for En {
    type Tile = Tile;
    type Vfx = Vfx;

    fn repr(&self) -> <<Self as Entity>::Tile as bn::Tile>::Repr {
        let ch = if !self.dormant {
            let mut ch = self.ch;
            // Highlight if about to act.
            if !self.is_player
                && (unsafe { GLOBAL_TIME % self.delay as u32 == (self.delay - 1) as u32 } || self.delay == 1)
                && self.vel.is_none()
            {
                ch = ch.on_red();
            }
            ch
        } else {
            ' '.stylize()
        };
        ch
    }

    fn update(&self, cmd: &mut bn::Commands<'_, Self>, pos: Point)
    where
        Self: Sized,
    {
        if self.is_dead()
            && let Special::Not = self.special
        {
            cmd.queue(bn::Cmd::new_here().delete_entity());
            if self.is_player {
                unsafe { DEAD = true }
            } else {
                unsafe { ENEMIES_REMAINING -= 1 }
            }
            return;
        }

        // Special entities
        match self.special {
            Special::WallSentry => {
                // This getting the chance to act means there are no enemies, so time to die.
                cmd.queue(bn::Cmd::new_here().delete_entity());

                return;
            }

            Special::Not => (),
        }

        let mut nx = None;
        let mut acted = false;

        // Check there is nothing at the given pos.
        let verify_pos = |cmd: &bn::Commands<'_, Self>, pos: Point| match cmd.get_map(pos) {
            Some(t) => match cmd.get_ent(pos) {
                Some(_e) => false,
                None => !t.blocking,
            },
            None => false,
        };

        // Checks there is nothing blocking between the two points.
        let verify_line = |cmd: &bn::Commands<'_, Self>, from: Point, to: Point| {
            Point::plot_line(from, to)
                .skip(1)
                .all(|p| verify_pos(cmd, p))
        };

        // Return the closest entity to the given position that is visible and within
        // range, if there is one.
        let get_closest = |cmd: &bn::Commands<'_, Self>, to: Point, range: u32, friendly: bool| {
            cmd.get_entities()
                .filter(move |(p, e)| {
                    e.is_player == friendly
                        && **p != to
                        && verify_line(cmd, to, **p)
                        && to.dist_squared(**p) as u32 <= range * range
                })
                .min_by_key(move |(p, _e)| to.dist_squared(**p))
                .map(|v| *v.0)
        };

        // Set all the acted values of entities to false.
        let update_entities = |cmd: &mut bn::Commands<'_, Self>| {
            let mut cmds = Vec::new();
            for (p, _e) in cmd.get_entities() {
                let p = *p;
                if p == pos {
                    continue;
                }

                cmds.push(bn::Cmd::new_on(p).modify_entity(Box::new(|e: &mut En| e.acted = false)));
            }
            cmd.queue_many(cmds);
        };

        // Perform a melee attack in the given direction, unless a ranged
        // attack is specified. In that case, the ranged attack at the provided index occurs.
        let do_attack =
            |cmd: &mut bn::Commands<'_, Self>, dir: Point, is_ranged: bool, atk_idx: usize| {
                let mut positions = Vec::new();
                let effects: &[Effect] = if is_ranged {
                    let atk = &self.atks.ranged_atks[atk_idx];
                    positions.push(pos + dir);
                    &atk.effects
                } else {
                    let atk = &self.atks.melee_atks[&dir][atk_idx];
                    for (p, v) in atk.fx.iter() {
                        cmd.queue(bn::Cmd::new_on(*p + pos).create_effect(v.clone()));
                    }
                    positions.extend(atk.place.iter().map(|p| *p + pos));
                    &atk.effects
                };

                for target in positions.into_iter() {
					for ef in effects {
						match ef {
							Effect::DoDmg(dmg_inst) => {
								let dmg_inst = *dmg_inst;
								let hit = rand::random_bool(dmg_inst.acc);
								
								// Draw line with closure for ranged attacks and display hit_fx if necessary.
								if is_ranged {
									let atk = &self.atks.ranged_atks[atk_idx];
									let mut line: Vec<Point> = Point::plot_line(pos, target).skip(1).collect();
									line.push(target);
									for (p, v) in (atk.line_fx)(hit, line) {
										cmd.queue(bn::Cmd::new_on(p).create_effect(v));
									}
								}
								
								if hit {
									// Apply damage.
									cmd.queue(bn::Cmd::new_on(target).modify_entity(Box::new(
										move |e: &mut En| {
											e.apply_dmg(dmg_inst);
										},
									)));
								} else if !is_ranged {
									cmd.queue(
										bn::Cmd::new_on(target)
											.create_effect(self.atks.melee_atks[&dir][atk_idx].miss_fx.clone()),
									);
								}
							}
							Effect::Other(clos) => cmd.queue_many(clos(pos, target, &*cmd)),
						}
					}
                }
            };

        // Apply velocity if necessary.
        if let Some(v) = self.vel {
            let cur_nx = v + pos;
            let mut stop = false;

            if verify_pos(cmd, cur_nx) {
                let t = cmd.get_map(cur_nx).unwrap();
                nx = Some(cur_nx);
                if !t.slippery {
                    stop = true;
                }
            } else {
                stop = true;
            }

            cmd.queue(
                bn::Cmd::new_here().modify_entity(Box::new(move |e: &mut En| {
                    if stop {
                        e.vel = None;
                    } else if e.is_player {
                        unsafe { GLOBAL_TIME += 1 }
                    }
                    e.acted = true;
                })),
            );
        } else {
            if self.is_player {
                let cur_nx = pos + unsafe { DIR };
                match unsafe { ACTION } {
                    ActionType::TryMove => {
                        if !cmd.get_map(cur_nx).unwrap().blocking {
                            // Check if there are any attacks that hit something in this direction,
                            // and if so, do the first one.
                            if let Some(atks) = self.atks.melee_atks.get(&unsafe { DIR }) {
                                'outer: for atk in atks.iter() {
                                    for (n, atk_pos) in atk.place.iter().enumerate() {
                                        if let Some(e) = cmd.get_ent(*atk_pos + pos)
                                            && !e.is_player
											&& !e.dormant
                                        {
                                            do_attack(cmd, unsafe { DIR }, false, n);
                                            acted = true;
                                            break 'outer;
                                        }
                                    }
                                }
                            }

                            // If there has been no action, move if there is no entity.
                            if !acted {
                                let no_ent = cmd.get_ent(cur_nx).is_none();
                                if no_ent || unsafe { ENEMIES_REMAINING == 0 } {
                                    // Displace the entity if it generates next to a door.
                                    if !no_ent {
                                        cmd.queue(
                                            bn::Cmd::new_on(cur_nx)
                                                .move_to(cur_nx + unsafe { DIR }),
                                        );
                                    }
                                    nx = Some(cur_nx);
                                    acted = true;
                                }
                            }
                        }
                    }
                    ActionType::Fire(idx) => {
                        // Verify there is an attack at idx before using it.
                        if self.atks.ranged_atks.len() > idx {
                            let range = self.atks.ranged_atks[idx].range;
                            if let Some(p) = get_closest(cmd, pos, range, false) {
                                acted = true;
                                do_attack(cmd, p - pos, true, idx)
                            }
                        }
                    }
                    ActionType::Wait => acted = true,
                }

                if acted {
                    unsafe { GLOBAL_TIME += 1 }
                    update_entities(cmd);
                }
            } else {
                acted = true;
                let mut attack = false;
                let mut range = false;
                let mut idx = 0;
                let mut dir = Point::ORIGIN;

                let goals = self
                    .atks
                    .find_attack_positions(unsafe { PLAYER })
                    .into_iter()
                    .filter(|p| verify_pos(&cmd, *p));
                let r_atk = self.atks.ranged_atks.get(0);

                // Check for melee attack.
                if let Some((atk_dir, i)) = self.atks.melee_hit_from(pos, unsafe { PLAYER }) {
                    range = false;
                    attack = true;
                    dir = atk_dir;
                    idx = i;
                // Then check ranged.
                } else if let Some(r) = r_atk
                    && let Some(_p) = get_closest(cmd, pos, r.range, true)
                {
                    range = true;
                    attack = true;
                    dir = unsafe { PLAYER - pos };
                // Try to pathfind.
                } else if let Some(path) =
                    cmd.pathfind(pos, goals, 20, |p| verify_pos(&cmd, p), &self.movement)
                {
                    match path.get(1) {
                        Some(path_pos) if *path_pos != unsafe { PLAYER } => nx = Some(*path_pos),
                        _ => attack = true,
                    }
                }

                if attack {
                    do_attack(cmd, dir, range, idx);
                }

                if acted {
                    cmd.queue(
                        bn::Cmd::new_here().modify_entity(Box::new(|e: &mut En| e.acted = true)),
                    );
                }
            }
        }

        if let Some(nx) = nx {
            cmd.queue(bn::Cmd::new_here().move_to(nx));

            // Do anything that the tile wants from us.
            if let Some(t) = cmd.get_map(nx) {
                let slip = t.slippery;

                if let Some(ref ef) = t.step_effect {
                    cmd.queue_many(ef(nx, &*cmd));
                }
                if slip {
                    cmd.queue(
                        bn::Cmd::new_here()
                            .modify_entity(Box::new(move |e: &mut En| e.vel = Some(nx - pos))),
                    );
                }
            }

            if self.is_player {
                unsafe { PLAYER = nx }
                update_entities(cmd);
            }

            // Check this is a door, and reveal the room if we move into it.
            if self.is_player
                && let Some(t) = cmd.get_map(pos)
            {
                // Door check.
                if let Some((room1, room2)) = &t.door {
                    let rect = if room1.contains(nx) { room1 } else { room2 };
                    let mut doors = Vec::new();

                    // Iterate over all cells of the room and reveal them.
                    for p in rect.cells() {
                        cmd.queue(
                            bn::Cmd::new_on(p)
                                .modify_tile(Box::new(|t: &mut Tile| t.revealed = true)),
                        );

                        // Mark the position for locking if it is a door.
                        if let Some(t) = cmd.get_map(p)
                            && t.door.is_some()
                        {
                            doors.push(p);
                        }

                        // Wake everyone up.
                        if p != pos
                            && let Some(_e) = cmd.get_ent(p)
                        {
                            cmd.queue(bn::Cmd::new_on(p).modify_entity(Box::new(
                                move |e: &mut En| {
                                    e.dormant = false;
									e.acted = true;
                                },
                            )));
                            unsafe { ENEMIES_REMAINING += 1 };
                        }
                    }

                    if doors.len() != 0 && unsafe { ENEMIES_REMAINING != 0 } {
                        // Lock the doors
                        for door in doors {
                            cmd.queue(bn::Cmd::new_on(door).create_entity(En::new(
                                1,
                                false,
                                255,
                                'x'.yellow(),
                                Special::WallSentry,
                                Vec::new(),
                                AtkPat::empty(),
                                false,
                            )));
                        }
                    }
                }
            }
        }
    }

    fn priority(&self) -> u32 {
        unsafe {
            match self.special {
                Special::Not => {
                    if self.dormant {
                        0
                    } else if self.is_dead() {
                        u32::MAX
                    } else if self.vel.is_some() && !self.acted {
                        3
                    } else if self.is_player {
                        1
                    } else if GLOBAL_TIME % self.delay as u32 == 0 && !self.acted {
                        2
                    } else {
                        0
                    }
                }
                Special::WallSentry => {
                    if ENEMIES_REMAINING == 0 {
                        u32::MAX
                    } else {
                        0
                    }
                }
            }
        }
    }
}
