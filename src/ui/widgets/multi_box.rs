use super::*;

/// Stores multiple states that can be switched between.
#[derive(Clone)]
pub struct MultiBox {
    /// Position on the screen of the box.
    screen_pos: Point,
    /// Current state in use.
    state: usize,
    /// Each state of the box.
    pub states: Vec<windowed::Window<StyleCh>>,
    /// Whether or not this box cares for scrolling.
    scrolls: bool,
}

impl MultiBox {
    const ATTACK_BOX_SIZE: u16 = 7;
    const INFO_WID: u16 = 23;
    const INFO_HGT: u16 = 12;
    const INFO_X_OFF: u16 = 14;
    const INFO_Y_OFF: u16 = 1;
    const KILLS_POS: Point = Point::new(2, 5);
    const HP_POS: Point = Point::new(2, 1);
    const FLOORS_POS: Point = Point::new(2, 2);
    const MOVES_POS: Point = Point::new(2, 3);
    const DESC_POS: Point = Point::new(2, 9);

    /// Construct an empty multi box.
    pub fn new(screen_pos: Point, scrolls: bool) -> Self {
        Self {
            screen_pos,
            state: 0,
            states: Vec::new(),
            scrolls,
        }
    }

    /// Return a reference to the current state.
    fn get_state(&self) -> &windowed::Window<StyleCh> {
        &self.states[self.state]
    }

    /// Adds a new state on to the end.
    pub fn add_state(&mut self, state: windowed::Window<StyleCh>) {
        self.states.push(state);
    }

    /// Create an info box using the given template and metadata and add it to this multi box.
    pub fn mk_info(
        &mut self,
        temp: &EntityTemplate,
        meta: &TempMeta,
        kills: u32,
        outline_ch: StyleCh,
        desc: &str
    ) {
        let mut info_win = windowed::Window::new(self.screen_pos);

        let win_centre = Point::new(
            (Self::INFO_X_OFF + (Self::ATTACK_BOX_SIZE / 2) + 1) as i32,
            (Self::INFO_Y_OFF + (Self::ATTACK_BOX_SIZE / 2)) as i32
        );

        // Populate the box.
        for _ in 0..Self::INFO_HGT-1 {
            info_win.data.push(vec![' '.stylize(); (Self::INFO_WID-2) as usize]);
        }
        info_win.outline_with(outline_ch);

        let add_line = |
            txt: &str,
            pos: Point,
            clr: style::Color,
            info_win: &mut windowed::Window<style::StyledContent<char>>
        | {
            for (x, ch) in txt.chars().enumerate() {
                info_win.data[pos.y as usize][pos.x as usize + x] = ch.with(clr);
            }
        };
        
        let add_lines = |
            txt: &str,
            pos: Point,
            clr: style::Color,
            len: usize,
            is_words: bool,
            info_win: &mut windowed::Window<style::StyledContent<char>>
        | {
            let mut cur_y = pos.y as usize;
            let mut x = 0;
            for wrd in txt.split(" ") {
                // Automatically put words on the next line if they are two big.
                if is_words {
                    let used = x - (cur_y - pos.y as usize) * len;
                    let rem = len - used;
                    if wrd.len() + used >= len {
                        cur_y += 1;
                        x += rem;
                    }
                }
                for ch in wrd.chars() {
                    let this_x = x + pos.x as usize - (cur_y - pos.y as usize) * len;
                    info_win.data[cur_y][this_x] = ch.with(clr);
                    if this_x > len && !is_words {
                        cur_y += 1;
                    }
                    x += 1;
                }
                x += 1;
            }
            
            cur_y - pos.y as usize
        };
        
        // Display enemy max health.
        add_line(&format!("Max HP:{}", temp.max_hp), Self::HP_POS, style::Color::Red, &mut info_win);

        // Display the floors it is found on.
        add_line(&format!("Floors:{}-{}", meta.floor_rang.start(), meta.floor_rang.end()), Self::FLOORS_POS, style::Color::White, &mut info_win);
        
        // Display enemy move sequence.
        let mut mvs = String::new();
        for ac in temp.actions.iter() {
            mvs = format!("{mvs}{ac}");
        }
        let extra = add_lines(&mvs, Self::MOVES_POS, style::Color::White, 11, false, &mut info_win);

        // Display the number of times this enemy has been killed.
        add_line(&format!("Killed:{kills}"), Self::KILLS_POS + Point::new(0, extra as i32), style::Color::White, &mut info_win);

        // Create graphic displaying the attack pattern and movement of the enemy.
        let damages: HashMap<Point, i32> = temp.atks.damage_map(Point::ORIGIN);

        for y in 0..=Self::ATTACK_BOX_SIZE {
            for x in 0..=Self::ATTACK_BOX_SIZE {
                let pos = Point::new((x + Self::INFO_X_OFF) as i32, (y + Self::INFO_Y_OFF) as i32);

                let mut ch = '.'.stylize();
                if x == 0 || y == Self::ATTACK_BOX_SIZE {
                    ch = outline_ch;
                }
                if pos == win_centre {
                    ch = temp.ch;
                } else if let Some(&dmg) = damages.get(&(win_centre - pos)) {
                    ch = if dmg >= 0 {
                        char::from_digit(dmg as u32, 16).unwrap().red()
                    } else {
                        char::from_digit(-dmg as u32, 16).unwrap().green()
                    };
                }
                if temp.movement.contains(&(pos - win_centre)) {
                    ch = ch.on(style::Color::Rgb { r: 255, g: 190, b: 0 });
                }
                info_win.data[(y + Self::INFO_Y_OFF) as usize][(x + Self::INFO_X_OFF) as usize] = ch;
            }
        }

        for x in 0..Self::INFO_WID {
            info_win.data[Self::ATTACK_BOX_SIZE as usize + 1][x as usize] = outline_ch.clone();
        }

        // Display the description.
        add_lines(
            desc,
            Self::DESC_POS,
            style::Color::White,
            Self::INFO_WID as usize - 4,
            true,
            &mut info_win
        );
        self.states.push(info_win);
    }
}

impl UiElement for MultiBox {
    fn receive(&mut self, data: &str) {
        for (n, seg) in data.split(' ').enumerate() {
            if n == 0 {
                if seg != "switch" {
                    return;
                }
            } else if let Ok(new) = seg.parse() {
                self.state = new;
            }
        }
    }

    // Not intended to be activated.
    fn activate(&mut self) -> Vec<Event> {
        vec![Event::Null]
    }

    fn priority(&self) -> i32 {
        1
    }

    // No text associated.
    fn get_text(&self) -> String {
        String::new()
    }

    fn true_pos(&self) -> Point {
        self.screen_pos
    }

    // Not intended to be selected.
    fn toggle_hover(&mut self) {}

    fn display_into(&self, win: &mut windowed::Window<StyleCh>, offset: Point) {
        let cur_state = self.get_state();

        for (y, row) in cur_state.data.iter().enumerate() {
            for (x, ch) in row.iter().enumerate() {
                let mut pos = Point::new(x as i32, y as i32) + self.screen_pos;

                if self.scrolls {
                    pos = pos + offset;
                }

                if let Some(rw) = win.data.get_mut(pos.y as usize)
                    && let Some(win_ch) = rw.get_mut(pos.x as usize) 
                {
                    *win_ch = ch.clone(); 
                }
            }
        }
    }
}

