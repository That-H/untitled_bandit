use crate::Point;
use rand::{Rng, prelude::*};
use rect::Rect;
use std::collections::HashMap;

/// A singular cell in a map.
#[derive(Debug)]
pub enum Cell {
    /// A wall that is part of rooms with the given ids.
    Wall(Vec<usize>),
    /// A cell inside the room of the given id.
    Inner(usize),
    /// A cell joining the two room ids.
    Door(usize, usize),
}

impl Cell {
    fn from_id(id: usize, is_wall: bool) -> Self {
        if is_wall {
            Self::Wall(vec![id])
        } else {
            Self::Inner(id)
        }
    }

    fn add_id(&mut self, id: usize) {
        if let Self::Wall(ids) = self {
            ids.push(id);
        }
    }
}

impl From<Cell> for bool {
    fn from(val: Cell) -> Self {
        match val {
            Cell::Wall(_) => true,
            Cell::Inner(_) => false,
            Cell::Door(_, _) => false,
        }
    }
}

impl From<Cell> for usize {
    fn from(val: Cell) -> Self {
        match val {
            Cell::Wall(ids) => ids[0] + 1,
            Cell::Inner(_id) => 0,
            Cell::Door(_id1, _id2) => 0,
        }
    }
}

/// Generate a new rectangle in the given map (rect list and cell hashmap).
/// Will not create a new rectangle with a door to an illegal host.
pub fn gen_rect_in<R: Rng>(
    rects: &mut Vec<Rect>,
    occupied: &mut HashMap<Point, Cell>,
    rng: &mut R,
    min_size: i32,
    max_size: i32,
    illegal_hosts: &[usize],
) {
    // Return the id of the first rect found to overlap with the given one that is not exempt.
    let overlaps = |r: &Rect, rects: &[Rect], exempt: &[usize]| -> Option<usize> {
        rects
            .iter()
            .enumerate()
            .find(|(n, tst)| !exempt.contains(n) && r.overlaps(tst))
            .map(|(n, _r)| n)
    };

    let r = rects.len();
    let mut host = 0;
    let mut init_pos: Point;
    let rect = if r == 0 {
        init_pos = Point::new(-max_size / 2, -max_size / 2);
        // Initial square.
        Rect::new(-max_size / 2, max_size / 2, max_size, max_size)
    } else {
        loop {
            let mut left = rng.random_range(-max_size..max_size) * max_size;
            let mut top = rng.random_range(-max_size..max_size) * max_size;
            let max_wid = rng.random_range(min_size..=max_size);
            let max_hgt = rng.random_range(min_size..=max_size);
            host = loop {
                let new_host = rng.random_range(0..rects.len());
                if !illegal_hosts.contains(&new_host) {
                    break new_host;
                }
            };
            let rand_rect = &rects[host];
            let corners = rand_rect.corners();
            let mut edges: Vec<Point> = rand_rect.edges().collect();
            edges.shuffle(rng);

            for pos in edges.into_iter() {
                if corners.contains(&pos) {
                    continue;
                } else if let Some(c) = occupied.get(&pos)
                    && let Cell::Wall(ids) = c
                    && ids.len() == 1
                {
                    left = pos.x;
                    top = pos.y;
                    break;
                }
            }

            init_pos = Point::new(left, top);

            let mut exempt = vec![host];
            let mut new_rect = Rect::new(left, top, 1, 1);
            let mut dirs: Vec<(Point, bool)> = Point::ORIGIN
                .get_all_adjacent()
                .into_iter()
                .map(|p| (p, true))
                .collect();

            // Get initial legal directions.
            for (dir, allowed) in dirs.iter_mut() {
                *allowed = match occupied.get(&(init_pos + *dir)) {
                    Some(c) => match c {
                        Cell::Wall(ids) => {
                            ids.iter().filter(|id| !exempt.contains(id)).count() <= 1
                        }
                        _ => false,
                    },
                    None => true,
                };
            }

            // Repeatedly expand the rect in allowed directions, disallowing them as necessary.
            let mut cont = true;
            while cont {
                cont = false;
                for (dir, allowed) in dirs.iter_mut() {
                    if *allowed {
                        cont = true;
                    } else {
                        continue;
                    }

                    if (new_rect.wid == max_wid && dir.x != 0)
                        || (new_rect.hgt == max_hgt && dir.y != 0)
                    {
                        *allowed = false;
                        continue;
                    }

                    new_rect.expand(*dir);

                    if let Some(id) = overlaps(&new_rect, &rects, &exempt) {
                        *allowed = false;
                        exempt.push(id);
                    }
                }
            }

            // If the size is right, we are done here.
            if new_rect.wid >= min_size && new_rect.hgt >= min_size {
                break new_rect;
            }
        }
    };

    let top = rect.top;
    let bottom = rect.bottom();
    let left = rect.left;
    let right = rect.right();
    for p in rect.cells() {
        let mut wall = p.y == top || p.y == bottom || p.x == left || p.x == right;
        if r != 0 && p == init_pos {
            wall = false;
        }

        occupied
            .entry(p)
            .and_modify(|c| {
                if wall {
                    c.add_id(r);
                } else {
                    *c = Cell::Door(r, host);
                }
            })
            .or_insert(Cell::from_id(r, wall));
    }

    rects.push(rect);
}

/// Create various connected rectangles. Returns a map of cells,
/// and all the rooms (represented by rectangles). Ids of the rectangles
/// are just the index of them in the vector.
pub fn map_gen<R: Rng>(
    rect_count: u32,
    max_size: i32,
    min_size: i32,
    rng: &mut R,
) -> (HashMap<Point, Cell>, Vec<Rect>) {
    let mut rects: Vec<Rect> = Vec::new();
    // let mut rng = rand::rngs::StdRng::from_seed([56; 32]);

    let mut occupied: HashMap<Point, Cell> = HashMap::new();

    for _r in 0..rect_count {
        gen_rect_in(&mut rects, &mut occupied, rng, min_size, max_size, &[]);
    }

    (occupied, rects)
}
