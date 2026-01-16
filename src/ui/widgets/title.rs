use super::*;
use crossterm::terminal; 
use std::time::Duration;
use std::cell::RefCell;

/// Title that does not care for the window it is supposed to be confined to.
#[derive(Clone)]
pub struct Title {
    /// Position where the title is.
    pub screen_pos: Point,
    /// The text that the title uses.
    win: windowed::Window<StyleCh>,
    fades: Option<Duration>,
    displayed: RefCell<bool>,
}

impl Title {
    /// Construct a new title instance.
    pub fn new(screen_pos: Point, txt: String, fades: Option<Duration>) -> Self {
        let mut win = windowed::Window::new(screen_pos);

        for ln in txt.lines() {
            let mut chs = Vec::new();
            for ch in ln.chars() {
                chs.push(ch.stylize());
            }
            win.data.push(chs);
        }

        Self {
            screen_pos,
            win,
            fades,
            displayed: RefCell::new(false),
        }
    }
}

impl UiElement for Title {
    fn activate(&mut self) -> Vec<Event> {
        vec![Event::Null]
    }

    fn priority(&self) -> i32 {
        100
    }

    fn receive(&mut self, data: &str) {
        let mut handle = io::stdout();

        if data == "clr" {
            for y in 0..self.win.data.len() {
                let _ = queue!(
                    handle,
                    cursor::MoveTo(0, y as u16 + self.screen_pos.y as u16),
                    terminal::Clear(terminal::ClearType::CurrentLine),
                    style::Print(' ')
                );
            }
            *self.displayed.borrow_mut() = false;
            let _ = handle.flush();
        }
    }
    
    fn get_text(&self) -> String {
        String::new()
    }

    fn true_pos(&self) -> Point {
        self.screen_pos
    }

    fn display_into(&self, _win: &mut windowed::Window<StyleCh>, _offset: Point) {
        if *self.displayed.borrow() {
            return;
        } else {
            *self.displayed.borrow_mut() = true;
        }
        
        let mut handle = io::stdout();

        for (y, line) in self.win.data.iter().enumerate() {
            for (x, ch) in line.iter().enumerate() {
                let _ = queue!(handle, cursor::MoveTo(x as u16 + self.screen_pos.x as u16, y as u16 + self.screen_pos.y as u16), style::Print(ch));
            }

            if let Some(d) = self.fades {
                let _ = handle.flush();
                std::thread::sleep(d);
            }
        }

        let _ = handle.flush();
    }

    fn toggle_hover(&mut self) {}
}
