//! Contains some widgets for use with a UiContainer.

use super::*;
use paste::paste;

macro_rules! field_builder {
    ($struct_name:ident, $field:ident; $f_type:ty) => {
        paste! {
            #[doc="Set the `" $field "` of the `" $struct_name "` ."]
            pub fn [<set_ $field>](self, $field: $f_type) -> Self {
                Self {
                    $field,
                    ..self
                }
            }
        }
    };

    ($struct_name:ident, $field:ident) => {
        paste! {
            #[doc="Set the `" $field "` flag of the `" $struct_name "` ."]
            pub fn [<set_ $field>](self, $field: bool) -> Self {
                Self {
                    $field,
                    ..self
                }
            }
        }
    };
}

/// A button that has a selector when hovered and an event that is posted when it is activated.
#[derive(Clone, Debug)]
pub struct Button {
    txt: String,
    clr: style::Color,
    hover_clr: style::Color,
    selector: String,
    selector_clr: style::Color,
    event: Event,
    hover: bool,
    screen_pos: Point,
    static_len: bool,
}

impl Button {
    /// Return an empty button instance.
    pub fn empty_new() -> Self {
        Self::default()
    }

    field_builder! {Button, txt; String}
    field_builder! {Button, clr; style::Color}
    field_builder! {Button, hover_clr; style::Color}
    field_builder! {Button, selector_clr; style::Color}
    field_builder! {Button, selector; String}
    field_builder! {Button, event; Event}
    field_builder! {Button, static_len}
    field_builder! {Button, screen_pos; Point}

    /// Return the text that would be displayed by this button currently.
    fn get_text(&self) -> Vec<StyleCh> {
        let mut data = Vec::new();

        for ch in self.selector.chars() {
            let ch = if self.hover {
                ch.with(self.selector_clr)
            } else if self.static_len {
                ' '.stylize()
            } else {
                break;
            };
            data.push(ch);
        }

        for ch in self.txt.chars() {
            let clr = if self.hover { self.hover_clr } else { self.clr };
            data.push(ch.with(clr));
        }

        // Have to add extra spaces after the text to avoid ghost characters being left behind.
        if !self.hover {
            for _ in 0..self.selector.len() {
                data.push(' '.stylize());
            }
        }

        data
    }
}

impl Default for Button {
    fn default() -> Self {
        Self {
            txt: String::from(""),
            clr: style::Color::White,
            hover_clr: style::Color::Rgb {
                r: 255,
                g: 190,
                b: 0,
            },
            selector: String::from(">"),
            selector_clr: style::Color::Rgb {
                r: 255,
                g: 190,
                b: 0,
            },
            event: Event::Null,
            hover: false,
            screen_pos: Point::ORIGIN,
            static_len: false,
        }
    }
}

impl UiElement for Button {
    fn receive(&mut self, _data: &str) {}

    fn activate(&mut self) -> Event {
        self.event.clone()
    }

    fn display_into(&self, win: &mut windowed::Window<StyleCh>) {
        put_text(&self.get_text(), win, self.screen_pos);
    }

    fn toggle_hover(&mut self) {
        self.hover = !self.hover;
    }

    fn get_text(&self) -> String {
        self.txt.clone()
    }
}

/// Creates an outline around the window. Position is arbitrary. Ensures the resulting window
/// outline is rectangular.
pub struct Outline {
    ch: StyleCh,
    wid: usize,
}

impl Outline {
    /// Create a new outline.
    pub fn new(ch: StyleCh, wid: usize) -> Self {
        Self { ch, wid }
    }
}

impl UiElement for Outline {
    fn receive(&mut self, _data: &str) {}

    fn activate(&mut self) -> Event {
        Event::Null
    }

    fn display_into(&self, win: &mut windowed::Window<StyleCh>) {
        for row in &mut win.data {
            for _ in 0..self.wid - row.len() {
                row.push(' '.stylize());
            }
        }
        win.outline_with(self.ch);
    }

    fn toggle_hover(&mut self) {}

    fn priority(&self) -> i32 {
        i32::MAX
    }
}

/// Receives and stores text when activated. Deactivates when esc or enter is pressed.
#[derive(Clone)]
pub struct TextEntry {
    /// Text currently stored.
    pub txt: Vec<char>,
    /// Position in the txt that the cursor is at.
    cursor: usize,
    /// Whether it should be receiving text.
    active: bool,
    /// Whether it is just hovered or not.
    hover: bool,
    /// Colour it turns when hovered.
    hover_clr: style::Color,
    /// Colour it turns when active.
    active_clr: style::Color,
    /// Colour when it is not hovered.
    clr: style::Color,
    /// Colour the character at the cursor position is on.
    highlight_clr: style::Color,
    /// Maximum length of the entry box.
    len: usize,
    /// Character used in empty positions.
    empty: char,
    /// Position on the screen to display it.
    screen_pos: Point,
}

impl Default for TextEntry {
    fn default() -> Self {
        Self {
            txt: Vec::new(),
            cursor: 0,
            active: false,
            hover: false,
            hover_clr: style::Color::White,
            clr: style::Color::White,
            highlight_clr: style::Color::Yellow,
            active_clr: style::Color::DarkYellow,
            len: 7,
            empty: '_',
            screen_pos: Point::ORIGIN,
        }
    }
}

impl TextEntry {
    /// Create a new text entry box.
    pub fn new() -> Self {
        Self::default()
    }

    field_builder! {TextEntry, txt; Vec<char>}
    field_builder! {TextEntry, hover_clr; style::Color}
    field_builder! {TextEntry, active_clr; style::Color}
    field_builder! {TextEntry, clr; style::Color}
    field_builder! {TextEntry, highlight_clr; style::Color}
    field_builder! {TextEntry, len; usize}
    field_builder! {TextEntry, screen_pos; Point}

    /// Clears all text stored.
    pub fn clear(&mut self) {
        self.txt.clear();
    }

    /// Return the current representation of the entry box.
    fn get_text(&self) -> Vec<StyleCh> {
        let mut chars = Vec::new();
        let clr = if self.active {
            self.active_clr
        } else if self.hover {
            self.hover_clr
        } else {
            self.clr
        };

        for (n, ch) in self.txt.iter().enumerate() {
            let mut ch = ch.with(clr);
            if self.active && n == self.cursor {
                ch = ch.on(self.highlight_clr);
            }
            chars.push(ch);
        }

        for n in self.txt.len()..self.len {
            let mut ch = self.empty.with(clr);
            if self.active && n == self.cursor {
                ch = ch.on(self.highlight_clr);
            }

            chars.push(ch);
        }

        chars
    }
}

impl UiElement for TextEntry {
    fn receive(&mut self, _data: &str) {}

    fn activate(&mut self) -> Event {
        self.active = true;
        Event::Null
    }

    fn receive_text(&mut self, ev: event::KeyCode) -> bool {
        if !self.active {
            return false;
        }

        // Necessary to prevent navigation and text input happening at the same time on the frame
        // where text input finishes.
        let init = self.active;

        match ev {
            event::KeyCode::Char(c) => {
                if self.txt.len() < self.len {
                    self.txt.insert(self.cursor, c);
                    self.cursor += 1;
                }
            }
            event::KeyCode::Esc | event::KeyCode::Enter => self.active = false,
            event::KeyCode::Left => self.cursor -= 1,
            event::KeyCode::Right => self.cursor += 1,
            event::KeyCode::Backspace => {
                if self.txt.len() >= self.cursor {
                    if self.txt.len() == self.cursor {
                        self.txt.pop();
                    } else {
                        self.txt.remove(self.cursor);
                    }
                    if self.cursor != 0 {
                        self.cursor -= 1;
                    }
                }
            }
            _ => (),
        }

        if self.cursor == self.len {
            self.cursor -= 1;
        }

        init
    }

    fn toggle_hover(&mut self) {
        self.hover = !self.hover;
    }

    fn display_into(&self, win: &mut windowed::Window<StyleCh>) {
        put_text(&self.get_text(), win, self.screen_pos);
    }

    fn get_text(&self) -> String {
        self.txt.iter().collect()
    }
}

fn put_text(txt: &[StyleCh], win: &mut windowed::Window<StyleCh>, pos: Point) {
    let offset = pos.x as usize;

    for (n, ch) in win.data[pos.y as usize].iter_mut().enumerate() {
        if n >= offset {
            let idx = n - offset;
            if idx == txt.len() {
                break;
            } else {
                *ch = txt[idx];
            }
        }
    }
}
