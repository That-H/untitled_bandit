use crate::*;
use attacks::*;
use entity::*;

pub const PLAYER_CHARACTER: char = '@';
pub const PLAYER_COLOUR: style::Color = style::Color::Green;

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
        viking_atk.melee_atks.get_mut(dir).unwrap().push(atks[0].clone());
    }

    // Pull the target towards self, without damaging them.
    let mut wizardry = AtkPat::from_atks(MeleeAtk::bulk_new::<4>(
        vec![Effect::Other(|from, to, _map| {
            let disp = (from - to) / 2;
            let new = to + disp;

            unsafe {
                if PLAYER == to {
                    PLAYER = new;
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
        ],
        // Elites start here.
        vec![EntityTemplate {
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
        }],
    )
}
