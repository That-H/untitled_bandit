//! Contains all things to do with entities.
#![allow(static_mut_refs)]

use super::*;
use crate::REVEALED;
use crate::bn;
use attacks::*;
use bn::Entity;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::rc::Rc;
use std::sync::{LazyLock, RwLock};

/// Type of action the player will perform.
pub static mut ACTION: ActionType = ActionType::Wait;
/// Current position of player.
pub static mut PLAYER: Point = Point::new(0, 0);
/// Are you dead yet?
pub static mut DEAD: bool = false;
/// Number of enemies remaining in the current room.
pub static mut ENEMIES_REMAINING: usize = 0;
/// Number of enemies killed over the course of the run.
pub static mut KILLED: u32 = 0;
/// Number of actions taken by the player.
pub static mut GLOBAL_TIME: u32 = 0;
/// Number of actions taken by the player while there are still enemies alive.
pub static mut COMBAT_TIME: u32 = 0;
/// Points of damage dealt to enemies.
pub static mut DAMAGE_DEALT: u32 = 0;
/// Number of floors cleared.
pub static mut FLOORS_CLEARED: u32 = 0;
/// True when the floor should be regenerated.
pub static mut NEXT_FLOOR: bool = false;
/// List of all keys the player has collected.
pub static mut KEYS_COLLECTED: [u32; KEY_CLRS_COUNT] = [0; KEY_CLRS_COUNT];
/// Contains messages about what has occurred.
pub static LOG_MSGS: RwLock<Vec<LogMsg>> = RwLock::new(Vec::new());
/// Stack of all recently entered door positions.
pub static LAST_DOOR: RwLock<Option<Point>> = RwLock::new(None);
/// Walk through the waller.
pub static NO_CLIP: RwLock<bool> = RwLock::new(false);
/// Times each enemy has been killed.
pub static KILL_COUNTS: RwLock<LazyLock<HashMap<char, u32>>> = RwLock::new(LazyLock::new(|| {
    match save_file::load_kills() {
        Ok(map) => map,
        Err(why) => match why {
            puzzle_loader::LoadErr::NotFound => HashMap::new(),
            _ => panic!("{why}"),
        },
    }
}));

/// Contains the id of the puzzle if we are currently doing one. Not to be confused with a puzzle room.
pub static mut PUZZLE: Option<usize> = None;

pub const KEY_CLRS: [style::Color; 4] = [
    style::Color::DarkRed,
    style::Color::Green,
    style::Color::Yellow,
    style::Color::Blue,
];
pub const KEY_CLRS_COUNT: usize = KEY_CLRS.len();
const WALL_SENTRY_CHAR: char = '█';

/// Displays a log message.
#[derive(Clone)]
pub struct LogMsg {
    txt: String,
    t_stamp: u32,
}

impl LogMsg {
    /// Create a new message using GLOBAL_TIME and the given text.
    pub fn new(txt: String) -> Self {
        Self {
            txt,
            t_stamp: unsafe { GLOBAL_TIME },
        }
    }
}

impl fmt::Display for LogMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "T+{}: {}", self.t_stamp, self.txt)
    }
}

impl From<String> for LogMsg {
    fn from(val: String) -> Self {
        Self::new(val)
    }
}

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
    pub actions: Vec<ActionType>,
    pub movement: Vec<Point>,
    pub ch: style::StyledContent<char>,
    pub atks: AtkPat,
}

#[derive(Clone)]
pub struct En {
    /// Stores current and maximum hp of the entity.
    pub hp: Datum<u32>,
    /// Stores the pattern through which the entity cycles.
    pub actions: Vec<ActionType>,
    /// Index of the next action to use.
    pub count: usize,
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
    pub acted: bool,
    /// Contains a value if the entity is forced to move in a specific direction.
    pub vel: Option<Point>,
}

impl En {
    /// Creates an entity with the provided data.
    pub fn new(
        max_hp: u32,
        is_player: bool,
        actions: Vec<ActionType>,
        ch: style::StyledContent<char>,
        special: Special,
        movement: Vec<Point>,
        atks: AtkPat,
        dormant: bool,
    ) -> Self {
        Self {
            hp: Datum::new(max_hp),
            count: 0,
            actions,
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
            actions,
            movement,
            ch,
            atks,
        } = template.clone();

        Self::new(
            max_hp,
            is_player,
            actions,
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
        let can_count = if !self.is_player
            && let Special::Not = self.special
        {
            true
        } else {
            false
        };
        match dmg.dmg {
            DmgType::Heal(h) => {
                self.hp += h;
                false
            }
            DmgType::Dmg(d) => {
                if d > *self.hp {
                    if can_count {
                        unsafe {
                            DAMAGE_DEALT += *self.hp;
                        }
                    }
                    self.hp.set_to(0);
                    true
                } else {
                    if can_count {
                        unsafe {
                            DAMAGE_DEALT += d;
                        }
                    }
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
        if !self.dormant || *REVEALED.read().unwrap() {
            // Required as the player has no action queue.
            if self.is_player {
                return self.ch;
            }
            // Highlight if about to act.
            match self.actions.get(self.count).unwrap() {
                ActionType::Wait | ActionType::Pathfind => self.ch,
                _ => self.ch.on_red(),
            }
        } else {
            ' '.stylize()
        }
    }

    fn update(&self, cmd: &mut bn::Commands<'_, Self>, pos: Point)
    where
        Self: Sized,
    {
        if self.is_dead()
            && let Special::Not = self.special
        {
            // Write to the global kill counter for this enemy type if this is not a puzzle.
            if unsafe { PUZZLE.is_none() } {
                *KILL_COUNTS.write().unwrap().entry(*self.ch.content()).or_insert(0) += 1;
            }
            if self.is_player {
                unsafe { DEAD = true }
            } else {
                let mut handle = LOG_MSGS.write().unwrap();
                handle.push(format!("{} is dead", *self.ch.content()).into());
                unsafe {
                    ENEMIES_REMAINING -= 1;
                    KILLED += 1;
                }
                cmd.queue(bn::Cmd::new_here().delete_entity());
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

        let mut acted = false;
        let mut new_count = self.count + 1;

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
        let update_entities = move |cmd: &mut bn::Commands<'_, Self>| {
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
        let do_attack = |pos: Point,
                         cmd: &mut bn::Commands<'_, Self>,
                         dir: Point,
                         is_ranged: bool,
                         atk_idx: usize| {
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
                                let mut line: Vec<Point> =
                                    Point::plot_line(pos, target).skip(1).collect();
                                line.push(target);
                                for (p, v) in (atk.line_fx)(hit, line) {
                                    cmd.queue(bn::Cmd::new_on(p).create_effect(v));
                                }
                            }

                            if hit {
                                let ch = *self.ch.content();
                                if cmd.get_ent(target).is_some() {
                                    // Apply damage.
                                    cmd.queue(bn::Cmd::new_on(target).modify_entity(Box::new(
                                        move |e: &mut En| {
                                            let old = *e.hp;
                                            e.apply_dmg(dmg_inst);
                                            let mut handle = LOG_MSGS.write().unwrap();
                                            let e_ch = *e.ch.content();
                                            handle.push(
                                                format!(
                                                    "{} {} -> {}",
                                                    ch,
                                                    dmg_inst.total_dmg(),
                                                    e_ch
                                                )
                                                .into(),
                                            );
                                            if e_ch != WALL_SENTRY_CHAR {
                                                handle.push(
                                                    format!(
                                                        "{} hp: {}/{}->{}/{}",
                                                        e_ch, old, e.hp.max, *e.hp, e.hp.max
                                                    )
                                                    .into(),
                                                );
                                            }
                                        },
                                    )));
                                }
                            } else if !is_ranged {
                                cmd.queue(bn::Cmd::new_on(target).create_effect(
                                    self.atks.melee_atks[&dir][atk_idx].miss_fx.clone(),
                                ));
                            }
                        }
                        Effect::Other(clos) => cmd.queue_many(clos(pos, target, &*cmd)),
                    }
                }
            }
        };

        // Stupid shit to get a recursive closure.
        let handle_action_inner: Rc<
            RefCell<
                Box<dyn Fn(ActionType, &mut bn::Commands<En>, &En, Point) -> (Point, bool, usize)>,
            >,
        > = Rc::new(RefCell::new(Box::new(|_, _, _, _| {
            (Point::ORIGIN, false, 0)
        })));

        let handle_action = Rc::clone(&handle_action_inner);

        *(handle_action_inner.borrow_mut()) = Box::new(
            move |act: ActionType,
                  cmd: &mut bn::Commands<En>,
                  cur_en: &En,
                  mut pos: Point|
                  -> (Point, bool, usize) {
                let mut acted = false;
                let mut nx: Option<Point> = None;
                let mut new_count = cur_en.count + 1;
                match act {
                    ActionType::TryMove(disp) => {
                        let cur_nx = pos + disp;
                        let clip = *NO_CLIP.read().unwrap();
                        let (unlockable, possible) = match cmd.get_map(cur_nx) {
                            Some(t) => {
                                let u = t.unlockable();
                                (u, !t.blocking || u || clip)
                            }
                            None => (false, if CHEATS { clip } else { false }),
                        };

                        if possible {
                            // Check if there are any attacks that hit something in this direction,
                            // and if so, do the first one.
                            if let Some(atks) = cur_en.atks.melee_atks.get(&disp) {
                                'outer: for atk in atks.iter() {
                                    for (n, atk_pos) in atk.place.iter().enumerate() {
                                        if let Some(e) = cmd.get_ent(*atk_pos + pos)
                                            && (e.is_player ^ cur_en.is_player)
                                            && !e.dormant
                                        {
                                            do_attack(pos, cmd, disp, false, n);
                                            acted = true;
                                            break 'outer;
                                        }
                                    }
                                }
                            }

                            // If there has been no action, move if there is no entity in the way.
                            if !acted {
                                let no_ent = cmd.get_ent(cur_nx).is_none();
                                if no_ent || unsafe { ENEMIES_REMAINING == 0 } {
                                    // Displace the entity if it generates next to a door.
                                    if !no_ent {
                                        cmd.queue(bn::Cmd::new_on(cur_nx).move_to(cur_nx + disp));
                                    }
                                    nx = Some(cur_nx);
                                    acted = true;
                                }
                                // Unlock the door.
                                if unlockable {
                                    nx = None;
                                    cmd.queue(
                                        bn::Cmd::new_on(cur_nx)
                                            .modify_tile(Box::new(|t: &mut Tile| t.unlock())),
                                    );
                                }
                            }
                        }
                    }
                    ActionType::TryMelee => {
                        // Check for melee attack against the player.
                        if let Some((atk_dir, i)) = self.atks.melee_hit_from(pos, unsafe { PLAYER })
                        {
                            acted = true;
                            do_attack(pos, cmd, atk_dir, false, i);
                        }
                    }
                    ActionType::ForceMelee(dir, idx) => {
                        // Always occurs, so this always counts as an action.
                        acted = true;
                        do_attack(pos, cmd, dir, false, idx);
                    }
                    ActionType::Fire(idx) => {
                        // Verify there is an attack at idx before using it.
                        if cur_en.atks.ranged_atks.len() > idx {
                            let range = cur_en.atks.ranged_atks[idx].range;
                            if let Some(p) = get_closest(cmd, pos, range, !cur_en.is_player) {
                                acted = true;
                                do_attack(pos, cmd, p - pos, true, idx)
                            }
                        }
                    }
                    ActionType::Pathfind => {
                        let goals = cur_en
                            .atks
                            .find_attack_positions(unsafe { PLAYER })
                            .into_iter()
                            .filter(|p| verify_pos(cmd, *p));

                        if let Some(path) =
                            cmd.pathfind(pos, goals, 20, |p| verify_pos(cmd, p), &cur_en.movement)
                        {
                            match path.get(1) {
                                Some(path_pos) if *path_pos != unsafe { PLAYER } => {
                                    nx = Some(*path_pos);
                                    acted = true;
                                }
                                _ => (),
                            }
                        }
                    }
                    ActionType::Wait => {
                        acted = true;
                    }
                    ActionType::Chain(first, fail) => {
                        (pos, acted, new_count) =
                            handle_action.borrow()((*first).clone(), cmd, cur_en, pos);
                        if !acted {
                            (pos, _, new_count) =
                                handle_action.borrow()((*fail).clone(), cmd, cur_en, pos);
                            acted = true;
                        }
                    }
                    ActionType::Jump(idx) => {
                        (pos, acted, new_count) =
                            handle_action.borrow()(cur_en.actions[idx].clone(), cmd, cur_en, pos);
                    }
                    ActionType::CondBranch(idx_t, idx_f, clos) => {
                        let nx_idx = if clos(&*cmd, self, pos) { idx_t } else { idx_f };
                        (pos, acted, new_count) = handle_action.borrow()(
                            cur_en.actions[nx_idx].clone(),
                            cmd,
                            cur_en,
                            pos,
                        );
                    }
                    ActionType::Arbitrary(clos) => {
                        let cmds = clos(&*cmd, self, pos);
                        cmd.queue_many(cmds);
                        acted = true;
                    }
                    ActionType::Multi(first, second) => {
                        (pos, _, _) = handle_action.borrow()((*first).clone(), cmd, cur_en, pos);
                        (pos, acted, new_count) =
                            handle_action.borrow()((*second).clone(), cmd, cur_en, pos);
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
                                bn::Cmd::new_here().modify_entity(Box::new(move |e: &mut En| {
                                    e.vel = Some(nx - pos)
                                })),
                            );
                        }
                    }

                    if self.is_player {
                        unsafe { PLAYER = nx }
                    }

                    // Check this is a door, and reveal the room if we move into it.
                    if self.is_player
                        && let Some(t) = cmd.get_map(pos)
                    {
                        // Door check.
                        if t.door {
                            // Record the door we just entered.
                            let mut write = LAST_DOOR.write().unwrap();
                            let mut should_write = true;

                            // If we just saw this door, don't write it again.
                            if let Some(dr) = *write
                                && dr != pos
                            {
                                // Check if the sentinel value is present. If it is, then we must
                                // be reverting to the previous door, so remove the sentinel and
                                // don't write the current position.
                                if dr == Point::ORIGIN {
                                    write.take();
                                    should_write = false;
                                }
                            }

                            if should_write {
                                write.replace(pos);
                            }

                            let mut doors = Vec::new();
                            // Flag to say whether or not we are locking all the doors due to
                            // an enemy being detected in the room.
                            let mut dooring = false;

                            // Stack of positions to reveal.
                            let mut rev_stack = vec![nx];
                            let mut revd = HashSet::new();

                            // Reveal all cells of the room via floodfill.
                            while let Some(p) = rev_stack.pop() {
                                if let Some(cl) = cmd.get_map(p) {
                                    // Already visited to ignore.
                                    if revd.contains(&p) {
                                        continue;
                                    } else {
                                        // Push adjacent cells if this one is eligible.
                                        if !cl.blocking && !cl.door {
                                            for adj in p.get_all_adjacent_diagonal() {
                                                rev_stack.push(adj);
                                            }
                                        }

                                        // Do the revealing.
                                        cmd.queue(bn::Cmd::new_on(p).modify_tile(Box::new(
                                            |t: &mut Tile| t.revealed = true,
                                        )));
                                        revd.insert(p);
                                    }
                                }

                                // Mark the position for locking if it is a door.
                                if let Some(t) = cmd.get_map(p)
                                    && t.door
                                {
                                    doors.push(p);
                                }

                                // Wake everyone up.
                                if p != pos
                                    && let Some(_e) = cmd.get_ent(p)
                                {
                                    dooring = true;
                                    // Hack to stop the game crashing when an entity is displaced
                                    // upon entering the room.
                                    let mut e_pos = p;
                                    if e_pos == nx {
                                        e_pos = nx - pos + e_pos;
                                    }
                                    cmd.queue(bn::Cmd::new_on(e_pos).modify_entity(Box::new(
                                        move |e: &mut En| {
                                            e.dormant = false;
                                            e.acted = true;
                                            unsafe { ENEMIES_REMAINING += 1 };
                                        },
                                    )));
                                }
                            }

                            if !doors.is_empty() && dooring {
                                // Lock the doors
                                for door in doors {
                                    cmd.queue(bn::Cmd::new_on(door).create_entity(En::new(
                                        // Little uranium reference for you there.
                                        92,
                                        false,
                                        vec![ActionType::Wait],
                                        WALL_SENTRY_CHAR.with(get_door_clr()),
                                        Special::WallSentry,
                                        Vec::new(),
                                        AtkPat::empty(),
                                        false,
                                    )));
                                }
                            }
                        }
                    }
                    pos = nx;
                }

                (pos, acted, new_count)
            },
        );

        // Apply velocity if necessary.
        if let Some(v) = self.vel {
            let cur_nx = v + pos;
            let mut stop = false;

            if verify_pos(cmd, cur_nx) {
                let t = cmd.get_map(cur_nx).unwrap();
                if !t.slippery {
                    stop = true;
                }
                handle_action_inner.borrow()(ActionType::TryMove(v), cmd, self, pos);
            } else {
                stop = true;
            }

            cmd.queue(
                bn::Cmd::new_here().modify_entity(Box::new(move |e: &mut En| {
                    if stop {
                        e.vel = None;
                    } else if e.is_player {
                        unsafe {
                            GLOBAL_TIME += 1;
                            if ENEMIES_REMAINING > 0 {
                                COMBAT_TIME += 1;
                            }
                        }
                    }
                    e.acted = true;
                })),
            );
        } else {
            let cur_act = if self.is_player {
                unsafe { ACTION.clone() }
            } else {
                self.actions[self.count].clone()
            };
            (_, acted, new_count) = handle_action_inner.borrow()(cur_act, cmd, self, pos);
        }

        // Increase global time if player, otherwise set the flag to prevent multi actions.
        if acted || !self.is_player {
            if self.is_player {
                unsafe {
                    GLOBAL_TIME += 1;
                    if ENEMIES_REMAINING > 0 {
                        COMBAT_TIME += 1;
                    }
                }
                // Prevents enemies from being allowed to act if we just walked in.
                if unsafe { ENEMIES_REMAINING != 0 } {
                    update_entities(cmd);
                }
            } else {
                cmd.queue(
                    bn::Cmd::new_here().modify_entity(Box::new(move |e: &mut En| {
                        e.acted = true;
                        e.count = new_count;
                        if e.count >= e.actions.len() {
                            e.count = 0;
                        }
                    })),
                );
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
                    } else if !self.acted {
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
