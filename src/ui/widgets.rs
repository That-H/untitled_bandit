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

    field_builder!{Button, txt; String}
    field_builder!{Button, clr; style::Color}
    field_builder!{Button, hover_clr; style::Color}
    field_builder!{Button, selector_clr; style::Color}
    field_builder!{Button, selector; String}
    field_builder!{Button, event; Event}
    field_builder!{Button, static_len}
    field_builder!{Button, screen_pos; Point}

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
            hover_clr: style::Color::Rgb { r: 255, g: 190, b: 0 },
            selector: String::from(">"),
            selector_clr: style::Color::Rgb { r: 255, g: 190, b: 0 },
            event: Event::Null,
            hover: false,
            screen_pos: Point::ORIGIN,
            static_len: false
        }
    }
}

impl UiElement for Button {
    fn receive(&mut self, _d1: usize, _d2: usize) {}

    fn activate(&mut self) -> Event {
        self.event.clone()
    }

    fn display_into(&self, win: &mut windowed::Window<StyleCh>) {
        let mut txt = self.get_text();

        for _ in 0..self.screen_pos.x {
            txt.insert(0, ' '.stylize());
        }

        win.data[self.screen_pos.y as usize] = txt;
    }

    fn toggle_hover(&mut self) {
        self.hover = !self.hover;
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
        Self { 
            ch,
            wid,
        }
    }
}

impl UiElement for Outline {
    fn receive(&mut self, _d1: usize, _d2: usize) {}

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
