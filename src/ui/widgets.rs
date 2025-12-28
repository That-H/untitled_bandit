use super::*;

/// A button that has a selector when hovered and an event that is posted when it is activated.
#[derive(Debug)]
pub struct Button {
    txt: String,
    clr: style::Color,
    hover_clr: style::Color,
    selector: String,
    selector_clr: style::Color,
    event: Event,
    hover: bool,
}

impl Button {
    /// Create a new button.
    pub fn new(
        txt: String,
        clr: style::Color,
        hover_clr: style::Color,
        selector: String,
        selector_clr: style::Color,
        event: Event,
    ) -> Self {
        Self {
            txt,
            clr,
            hover_clr,
            selector,
            selector_clr,
            event,
            hover: false,
        }
    }

    /// Return the text that would be displayed by this button currently.
    fn get_text(&self) -> Vec<StyleCh> {
        let mut data = Vec::new();

        if self.hover {
            for ch in self.selector.chars() {
                data.push(ch.with(self.selector_clr));
            }
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

impl UiElement for Button {
    fn receive(&mut self, _d1: usize, _d2: usize) {}

    fn activate(&mut self) -> Event {
        self.event.clone()
    }

    fn display_into(&self, win: &mut windowed::Window<StyleCh>, pos: Point) {
        let mut txt = self.get_text();

        for _ in 0..pos.x {
            txt.insert(0, ' '.stylize());
        }

        win.data[pos.y as usize] = txt;
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

    fn display_into(&self, win: &mut windowed::Window<StyleCh>, _pos: Point) {
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
