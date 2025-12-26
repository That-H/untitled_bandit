use crate::*;
use attacks::*;
use entity::*;
use rand::prelude::IndexedRandom;

pub const PLAYER_CHARACTER: char = '@';
pub const PLAYER_COLOUR: style::Color = style::Color::Green;
pub const RING_CHARS: [char; 6] = ['╔', '═', '╗', '║', '╝', '╚'];

/// Create an instance of the default attack pattern.
pub fn get_default_atks(
    dmg: u32,
    chars: impl IntoIterator<Item = char>,
    clr: style::Color,
) -> AtkPat {
    AtkPat::from_atks(MeleeAtk::bulk_new::<4>(
        vec![Effect::DoDmg(DmgInst::dmg(dmg, 1.0))],
        clr,
        7,
        Vfx::new_opaque('?'.stylize(), 7),
        chars.into_iter(),
    ))
}

/// Creates an attack with knockback.
pub fn get_hvy_atks(dmg: u32, chars: impl IntoIterator<Item = char>, clr: style::Color) -> AtkPat {
    AtkPat::from_atks(MeleeAtk::bulk_new::<4>(
        vec![
            Effect::DoDmg(DmgInst::dmg(dmg, 1.0)),
            Effect::Other(|from, to, _map| {
                vec![
                    bn::Cmd::new_on(to)
                        .modify_entity(Box::new(move |e: &mut En| e.vel = Some(to - from))),
                ]
            }),
        ],
        clr,
        7,
        Vfx::new_opaque('?'.stylize(), 7),
        chars,
    ))
}

/// Creates a holy attack.
pub fn get_holiness(dmg: u32, duration: usize) -> AtkPat {
    let mut atks = AtkPat::empty();
    let mut places = Point::ORIGIN.get_all_adjacent();
    let mut fx: Vec<(_, _)> = places
        .iter()
        .copied()
        .zip(THICC_FOUR_POS_ATK.into_iter())
        .map(|(p, ch)| (p, Vfx::new_opaque(ch.yellow(), duration)))
        .collect();
    places.push(Point::new(0, -2));
    fx.push((Point::new(0, -2), Vfx::new_opaque('║'.yellow(), duration)));
    fx.push((Point::new(0, 0), Vfx::new_opaque('╬'.yellow(), duration)));
    let melee = MeleeAtk::new(
        vec![Effect::DoDmg(DmgInst::dmg(dmg, 1.0))],
        places,
        fx,
        Vfx::new_opaque('?'.stylize(), 7),
    );
    atks.melee_atks.insert(Point::new(0, -1), vec![melee]);
    atks
}

/// Generate an attack against the edges of a square.
pub fn get_ring_attack(dmg: u32, clr: style::Color, edge_dist: i32, duration: usize) -> MeleeAtk {
    let mut atk = MeleeAtk::new(
        vec![Effect::DoDmg(DmgInst::dmg(dmg, 1.0))],
        Vec::new(),
        Vec::new(),
        Vfx::opaque_with_clr('?', style::Color::White, duration),
    );
    for x in -edge_dist..=edge_dist {
        for y in -edge_dist..=edge_dist {
            let up = if y == edge_dist {
                1
            } else if y == -edge_dist {
                -1
            } else {
                0
            };
            let right = if x == edge_dist {
                1
            } else if x == -edge_dist {
                -1
            } else {
                0
            };
            let ch = match (right, up) {
                (1, 0) | (-1, 0) => RING_CHARS[3],
                (0, 1) | (0, -1) => RING_CHARS[1],
                (1, 1) => RING_CHARS[2],
                (-1, 1) => RING_CHARS[0],
                (1, -1) => RING_CHARS[4],
                (-1, -1) => RING_CHARS[5],
                _ => continue,
            };
            let p = Point::new(x, y);
            atk.place.push(p);
            atk.fx.push((p, Vfx::opaque_with_clr(ch, clr, 8)));
        }
    }
    atk
}

/// Creates the player entity.
pub fn get_player() -> En {
    En::new(
        9,
        true,
        Vec::new(),
        PLAYER_CHARACTER.with(PLAYER_COLOUR),
        Special::Not,
        Point::ORIGIN.get_all_adjacent_diagonal(),
        get_default_atks(1, FOUR_POS_ATK, style::Color::Red),
        false,
    )
}

/// Create an explosion with the given manhattan radius.
pub fn get_explosion(dmg: u32, radius: i32, ch: StyleCh) -> MeleeAtk {
    let mut positions = Vec::new();

    for y in -radius..=radius {
        for x in -radius..=radius {
            let p = Point::new(x, y);
            if p == Point::ORIGIN {
                continue;
            }
            if Point::ORIGIN.manhattan_dist(p) <= radius {
                positions.push(p);
            }
        }
    }

    let mut fx = Vec::new();

    for pos in &positions {
        fx.push((
            *pos,
            Vfx::new_opaque(ch, 4 + Point::ORIGIN.manhattan_dist(*pos) as usize),
        ));
    }

    MeleeAtk::new(
        vec![
            Effect::DoDmg(DmgInst::dmg(dmg, 1.0)),
            Effect::Other(|_pos, _target, _map| vec![bn::Cmd::new_here().delete_entity()]),
        ],
        positions,
        fx,
        Vfx::new_opaque('?'.stylize(), 7),
    )
}

/// Create a missile entity that moves in a single direction. Explodes if in proximity
/// to the player at the start of its turn.
pub fn get_missile(dir: Point, clr: style::Color, explosion: MeleeAtk) -> En {
    let mut atk_pat = AtkPat::empty();
    atk_pat.melee_atks.insert(Point::ORIGIN, vec![explosion]);
    En::new(
        1,
        false,
        vec![ActionType::Chain(
            Box::new(ActionType::TryMelee),
            Box::new(ActionType::Chain(
                Box::new(ActionType::TryMove(dir)),
                Box::new(ActionType::ForceMelee(Point::ORIGIN, 0)),
            )),
        )],
        FOUR_POS_ATK[dir.dir()].with(clr),
        Special::Not,
        Vec::new(),
        atk_pat,
        false,
    )
}

/// Return a vector of entity templates for use in the game. First vector is for normal enemies,
/// second is for elite enemies.
pub fn get_templates() -> (Vec<EntityTemplate>, Vec<EntityTemplate>) {
    // Generate knight moves without typing them all out.
    let mut p1 = Point::new(2, 1);
    let mut p2 = Point::new(2, -1);

    let mut knight = Vec::new();

    for _ in 0..4 {
        knight.push(p1);
        knight.push(p2);
        p1.rotate_90_cw_ip();
        p2.rotate_90_cw_ip();
    }

    // Manhattan movement.
    let manhattan = Point::ORIGIN.get_all_adjacent();

    // Diagonal movement 1 tile.
    let mut diag = manhattan.clone();
    for p in diag.iter_mut() {
        *p = Point::new(p.x + p.y, p.y - p.x);
    }

    // Diagonal movement up to three spaces.
    let mut diag_plus = diag.clone();
    for p in diag.iter() {
        diag_plus.push(*p * 2);
        diag_plus.push(*p * 3);
    }

    // Manhattan movement with diagonal.
    let total = Point::ORIGIN.get_all_adjacent_diagonal();

    // All moves with a manhattan distance of 2.
    let mut viking_move = total.clone();
    for p in manhattan.iter() {
        viking_move.push(*p * 2);
    }

    // All moves exactly two king moves away.
    let mut ring = Vec::new();
    for y in -2..=2i32 {
        for x in -2..=2i32 {
            if y.abs() == 2 || x.abs() == 2 {
                ring.push(Point::new(x, y));
            }
        }
    }

    // Attacks any square in the ring move pattern for 1 damage.
    let mut ring_atk = AtkPat::empty();
    ring_atk.melee_atks.insert(
        Point::ORIGIN,
        vec![get_ring_attack(2, style::Color::Red, 2, 9)],
    );

    // Default attack pattern.
    let default_atks = get_default_atks(1, FOUR_POS_ATK, style::Color::Red);

    // Functionally identical to default attacks, but looks different.
    let weird_default = get_default_atks(1, ['☼'; 4], style::Color::Magenta);

    // Default attack pattern with double damage and knockback.
    let heavy_default_atks = get_hvy_atks(2, THICC_FOUR_POS_ATK, style::Color::Red);

    // Default attack pattern with diagonals included.
    let diagonal_atks = AtkPat::from_atks(MeleeAtk::bulk_new::<8>(
        vec![Effect::DoDmg(DmgInst::dmg(1, 1.0))],
        style::Color::Red,
        7,
        Vfx::new_opaque('?'.stylize(), 7),
        EIGHT_POS_ATK.into_iter(),
    ));

    // Like diagonal_atks, but without the default_atks in it.
    let mut pure_diag_atks = diagonal_atks.clone();
    for p in Point::ORIGIN.get_all_adjacent() {
        pure_diag_atks.melee_atks.remove(&p);
    }

    // Long default attack.
    let mut spear = default_atks.clone();

    for (_d, atks) in spear.melee_atks.iter_mut() {
        for atk in atks.iter_mut() {
            let pos = atk.place[0];
            atk.fx
                .push((pos * 2, Vfx::new_opaque(DIR_CHARS[pos.dir()].red(), 7)));
            for p in atk.place.iter_mut() {
                *p = *p * 2;
            }
        }
    }

    // Viking movement as an attack pattern.
    let mut viking_atk = diagonal_atks.clone();
    for (dir, atks) in spear.melee_atks.iter() {
        viking_atk
            .melee_atks
            .get_mut(dir)
            .unwrap()
            .push(atks[0].clone());
    }

    // Pull the target towards self, without damaging them.
    let mut wizardry = AtkPat::from_atks(MeleeAtk::bulk_new::<4>(
        vec![Effect::Other(|from, to, map| {
            let disp = (from - to) / 2;
            let new = to + disp;

            unsafe {
                if PLAYER == to {
                    PLAYER = new;
                }
                if map.get_ent(new).is_some() {
                    ENEMIES_REMAINING -= 1;
                    KILLED += 1;
                }
            }
            vec![bn::Cmd::new_on(to).move_to(new)]
        })],
        style::Color::Magenta,
        7,
        Vfx::new_opaque('?'.stylize(), 7),
        FOUR_POS_ATK.into_iter(),
    ));

    for (_d, atks) in wizardry.melee_atks.iter_mut() {
        for atk in atks.iter_mut() {
            let pos = atk.place[0];
            atk.fx.push((pos * 2, Vfx::new_opaque('*'.magenta(), 7)));
            for p in atk.place.iter_mut() {
                *p = *p * 2;
            }
        }
    }

    // Like wizardry, but with a weird default attack included.
    let mut wizardry_plus = wizardry.clone();

    for (k, v) in weird_default.melee_atks.iter() {
        wizardry_plus
            .melee_atks
            .get_mut(k)
            .unwrap()
            .append(&mut v.clone());
    }

    // Attacks of the final boss.
    let mut omegattacks = AtkPat::empty();
    omegattacks.melee_atks.insert(Point::ORIGIN, Vec::new());

    // Create concentric rings.
    for edge_dist in 1..=3 {
        omegattacks
            .melee_atks
            .get_mut(&Point::ORIGIN)
            .unwrap()
            .push(get_ring_attack(2, style::Color::Red, edge_dist, 10));
    }

    fn go_furthest(map: &bn::Map<En>, _en: &En, _pos: Point) -> Vec<bn::Cmd<En>> {
        let pl = unsafe { PLAYER };
        let mut max_dist = 0;
        let mut new_pos = Point::ORIGIN;
        for disp in Point::ORIGIN.get_all_adjacent() {
            let mut cur = pl;
            let mut dist = 0;
            while !map.get_map(cur).unwrap().blocking {
                dist += 1;
                cur = cur + disp;
                if let Some(e) = map.get_ent(cur)
                    && let Special::WallSentry = e.special
                {
                    break;
                }
            }
            cur = cur - disp;
            if dist > max_dist {
                max_dist = dist;
                new_pos = cur;
            }
        }

        vec![bn::Cmd::new_here().move_to(new_pos)]
    }

    fn pl_to_wall(map: &bn::Map<En>, dir: Point) -> Vec<bn::Cmd<En>> {
        let mut cur = unsafe { PLAYER };
        while !map.get_map(cur).unwrap().blocking {
            cur = cur + dir;
            if let Some(e) = map.get_ent(cur)
                && let Special::WallSentry = e.special
            {
                break;
            }
        }

        vec![bn::Cmd::new_here().move_to(cur)]
    }

    fn fire_laser(
        mut from: Point,
        map: &bn::Map<En>,
        dmg: u32,
        ch: StyleCh,
        dir: Point,
    ) -> Vec<bn::Cmd<En>> {
        let mut cmds = Vec::new();

        // To prevent it from checking the position the caster is on.
        from = from + dir;

        while !map.get_map(from).unwrap().blocking {
            if let Some(e) = map.get_ent(from) {
                if let Special::WallSentry = e.special {
                    break;
                } else {
                    cmds.push(
                        bn::Cmd::new_on(from).modify_entity(Box::new(move |e: &mut En| {
                            e.apply_dmg(DmgInst::dmg(dmg, 1.0));
                        })),
                    );
                }
            }
            cmds.push(bn::Cmd::new_on(from).create_effect(Vfx::new_opaque(ch, 10)));
            from = from + dir;
        }

        cmds
    }

    (
        vec![
            EntityTemplate {
                max_hp: 3,
                actions: vec![
                    ActionType::Wait,
                    ActionType::Chain(
                        Box::new(ActionType::TryMelee),
                        Box::new(ActionType::Pathfind),
                    ),
                ],
                movement: manhattan.clone(),
                ch: 'e'.stylize(),
                atks: default_atks.clone(),
            },
            EntityTemplate {
                max_hp: 4,
                actions: vec![
                    ActionType::Wait,
                    ActionType::Wait,
                    ActionType::Chain(
                        Box::new(ActionType::TryMelee),
                        Box::new(ActionType::Pathfind),
                    ),
                ],
                movement: manhattan.clone(),
                ch: 'h'.stylize(),
                atks: heavy_default_atks.clone(),
            },
            EntityTemplate {
                max_hp: 2,
                actions: vec![ActionType::Chain(
                    Box::new(ActionType::TryMelee),
                    Box::new(ActionType::Pathfind),
                )],
                movement: manhattan.clone(),
                ch: 'l'.stylize(),
                atks: spear.clone(),
            },
            EntityTemplate {
                max_hp: 2,
                actions: vec![
                    ActionType::Wait,
                    ActionType::Chain(
                        Box::new(ActionType::TryMelee),
                        Box::new(ActionType::Pathfind),
                    ),
                ],
                movement: knight.clone(),
                ch: 'k'.stylize(),
                atks: diagonal_atks.clone(),
            },
            EntityTemplate {
                max_hp: 3,
                actions: vec![ActionType::Multi(
                    Box::new(ActionType::Pathfind),
                    Box::new(ActionType::TryMelee),
                )],
                movement: diag.clone(),
                ch: 'b'.stylize(),
                atks: pure_diag_atks.clone(),
            },
            EntityTemplate {
                max_hp: 3,
                actions: vec![
                    ActionType::Wait,
                    ActionType::Chain(
                        Box::new(ActionType::TryMelee),
                        Box::new(ActionType::Pathfind),
                    ),
                ],
                movement: manhattan.clone(),
                ch: 'w'.stylize(),
                atks: wizardry_plus.clone(),
            },
            EntityTemplate {
                max_hp: 2,
                actions: vec![
                    ActionType::TryMove(Point::new(1, 0)),
                    ActionType::TryMove(Point::new(0, -1)),
                    ActionType::TryMove(Point::new(-1, 0)),
                    ActionType::TryMove(Point::new(0, 1)),
                ],
                movement: manhattan.clone(),
                ch: 'o'.stylize(),
                atks: default_atks.clone(),
            },
            EntityTemplate {
                max_hp: 2,
                actions: vec![
                    ActionType::Wait,
                    ActionType::Wait,
                    ActionType::Chain(
                        Box::new(ActionType::TryMelee),
                        Box::new(ActionType::Pathfind),
                    ),
                ],
                movement: viking_move.clone(),
                ch: 'v'.stylize(),
                atks: viking_atk.clone(),
            },
            EntityTemplate {
                max_hp: 2,
                actions: vec![
                    ActionType::Wait,
                    ActionType::Chain(
                        Box::new(ActionType::TryMelee),
                        Box::new(ActionType::Pathfind),
                    ),
                    ActionType::Chain(
                        Box::new(ActionType::TryMelee),
                        Box::new(ActionType::Pathfind),
                    ),
                ],
                movement: total.clone(),
                ch: 'g'.stylize(),
                atks: diagonal_atks.clone(),
            },
        ],
        // Capitals start here.
        vec![
            EntityTemplate {
                max_hp: 7,
                actions: vec![
                    ActionType::Pathfind,
                    ActionType::Pathfind,
                    ActionType::Multi(
                        Box::new(ActionType::Pathfind),
                        Box::new(ActionType::TryMelee),
                    ),
                    ActionType::Wait,
                ],
                movement: diag_plus.clone(),
                ch: 'B'.stylize(),
                atks: get_holiness(3, 15),
            },
            EntityTemplate {
                max_hp: 5,
                actions: vec![
                    ActionType::Wait,
                    ActionType::Wait,
                    ActionType::Wait,
                    ActionType::Chain(
                        Box::new(ActionType::TryMelee),
                        Box::new(ActionType::Multi(
                            Box::new(ActionType::Pathfind),
                            Box::new(ActionType::TryMelee),
                        )),
                    ),
                    ActionType::Chain(
                        Box::new(ActionType::TryMelee),
                        Box::new(ActionType::Multi(
                            Box::new(ActionType::Pathfind),
                            Box::new(ActionType::TryMelee),
                        )),
                    ),
                ],
                movement: manhattan.clone(),
                ch: 'E'.stylize(),
                atks: default_atks.clone(),
            },
            EntityTemplate {
                max_hp: 3,
                actions: vec![
                    ActionType::Wait,
                    ActionType::Chain(
                        Box::new(ActionType::TryMelee),
                        Box::new(ActionType::Pathfind),
                    ),
                    ActionType::Chain(
                        Box::new(ActionType::TryMelee),
                        Box::new(ActionType::Pathfind),
                    ),
                ],
                movement: ring.clone(),
                ch: 'O'.stylize(),
                atks: ring_atk.clone(),
            },
            EntityTemplate {
                max_hp: 10,
                actions: vec![
                    ActionType::Arbitrary(Box::new(|map, _en, pos| {
                        let pl = unsafe { PLAYER };
                        let mut possible = Vec::new();
                        for p in Point::ORIGIN.get_all_adjacent() {
                            let new = pl + p;
                            if !map.get_map(new).unwrap().blocking {
                                possible.push(new);
                            }
                        }
                        vec![
                            bn::Cmd::new_on(pos)
                                .move_to(*possible.choose(&mut rand::rng()).unwrap()),
                        ]
                    })),
                    ActionType::ForceMelee(Point::ORIGIN, 0),
                    ActionType::ForceMelee(Point::ORIGIN, 1),
                    ActionType::ForceMelee(Point::ORIGIN, 2),
                    ActionType::Arbitrary(Box::new(|map, _en, _pos| {
                        pl_to_wall(map, Point::new(1, 0))
                    })),
                    ActionType::Multi(
                        Box::new(ActionType::Arbitrary(Box::new(|map, _en, pos| {
                            fire_laser(pos, map, 3, '-'.red(), Point::new(-1, 0))
                        }))),
                        Box::new(ActionType::Arbitrary(Box::new(|map, _en, pos| {
                            let mut cmds = pl_to_wall(map, Point::new(0, 1));
                            cmds.push(bn::Cmd::new_on(pos).create_effect(Vfx::new_opaque('Ω'.white(), 9)));
                            cmds
                        }))),
                    ),
                    ActionType::Multi(
                        Box::new(ActionType::Arbitrary(Box::new(|map, _en, pos| {
                            fire_laser(pos, map, 3, '|'.red(), Point::new(0, -1))
                        }))),
                        Box::new(ActionType::Arbitrary(Box::new(|map, _en, pos| {
                            let mut cmds = pl_to_wall(map, Point::new(-1, 0));
                            cmds.push(bn::Cmd::new_on(pos).create_effect(Vfx::new_opaque('Ω'.white(), 9)));
                            cmds
                        }))),
                    ),
                    ActionType::Multi(
                        Box::new(ActionType::Arbitrary(Box::new(|map, _en, pos| {
                            fire_laser(pos, map, 3, '-'.red(), Point::new(1, 0))
                        }))),
                        Box::new(ActionType::Arbitrary(Box::new(|map, _en, pos| {
                            let mut cmds = pl_to_wall(map, Point::new(0, -1));
                            cmds.push(bn::Cmd::new_on(pos).create_effect(Vfx::new_opaque('Ω'.white(), 9)));
                            cmds
                        }))),
                    ),
                    ActionType::Arbitrary(Box::new(|map, _en, pos| {
                        fire_laser(pos, map, 3, '|'.red(), Point::new(0, 1))
                    })),
                ],
                movement: ring.clone(),
                ch: 'Ω'.stylize(),
                atks: omegattacks.clone(),
            },
        ],
    )
}
