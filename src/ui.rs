//! Contains a simple UI framework.

use crate::Point;
use crate::bn;
use bn::windowed;
use crossterm::{cursor, event, queue, style};
use std::collections::HashMap;
use std::io::{self, Write};
use style::Stylize;

type StyleCh = style::StyledContent<char>;

pub mod widgets;

/// Types that can be used as widgets in a UI window.
pub trait UiElement {
    /// Display the current element into the window.
    fn display_into(&self, win: &mut windowed::Window<StyleCh>);

    /// Toggle whether this element is being hovered over.
    fn toggle_hover(&mut self);

    /// Create an event when the element is activated by the enter key.
    fn activate(&mut self) -> Event;

    /// Do something with received data.
    fn receive(&mut self, d1: usize, d2: usize);

    /// Return a value indicating when the UiElement should be drawn. The highest value will be
    /// drawn last.
    fn priority(&self) -> i32 { 0 }
}

/// Something that can occur when an element is activated.
#[derive(Clone, Debug)]
pub enum Event {
    /// Quit the current runtime with a code.
    Exit(u32),
    /// Change to the scene at the given index.
    ChangeScene(usize),
    /// Broadcast some data to be read by other elements.
    Broadcast(usize, usize),
    /// Do nothing.
    Null,
}

/// Contains some UI elements. For use with a UiContainer.
pub struct Scene {
    elements: HashMap<Point, Box<dyn UiElement>>,
    cursor: Point,
    win: windowed::Window<StyleCh>,
    wid: usize,
    hgt: usize,
}

impl Scene {
    /// Create an empty scene.
    pub fn new(top_left: Point, wid: usize, hgt: usize) -> Self {
        Self {
            elements: HashMap::new(),
            cursor: Point::ORIGIN,
            win: windowed::Window::new(top_left),
            wid,
            hgt,
        }
    }

    /// Add a new element to the container. Automatically hovers it if that is where the cursor
    /// currently is. Position is purely for navigation; the screen position should be stored
    /// separately.
    pub fn add_element(&mut self, mut elem: Box<dyn UiElement>, pos: Point) {
        if self.cursor == pos {
            elem.toggle_hover();
        }
        self.elements.insert(pos, elem);
    }

    /// Move the cursor to the given position if there is an element there.
    pub fn move_cursor(&mut self, new_pos: Point) {
        if self.try_hover(new_pos) {
            self.try_hover(self.cursor);
            self.cursor = new_pos;
        }
    }

    /// Displace the cursor by the given vector if there is an element at the new position.
    pub fn disp_cursor(&mut self, disp: Point) {
        self.move_cursor(self.cursor + disp);
    }

    /// Draw the UI elements into the window.
    pub fn draw(&mut self) {
        self.win.data.clear();

        for _ in 0..self.hgt {
            self.win.data.push(vec![' '.stylize(); self.wid]);
        }

        let mut elems = Vec::new();

        for elem in self.elements.values() {
            elems.push(elem);
        }

        elems.sort_by_key(|e| e.priority());

        for elem in elems {
            elem.display_into(&mut self.win);
        }
    }

    /// Toggle the hover state of the element at the given position if there is one. Returns true
    /// if an element is hovered, otherwise false.
    fn try_hover(&mut self, pos: Point) -> bool {
        if let Some(elem) = self.elements.get_mut(&pos) {
            elem.toggle_hover();
            true
        } else {
            false
        }
    }
}

enum Nav {
    Move(Point),
    Activate,
    Null,
}

/// Contains various scenes that can be navigated between. Handles key presses.
pub struct UiContainer {
    scenes: Vec<Scene>,
    cur: usize,
}

impl UiContainer {
    /// Create an empty UI container.
    pub fn new() -> Self {
        Self {
            scenes: Vec::new(),
            cur: 0,
        }
    }

    /// Add the scene to the container in the last position.
    pub fn add_scene(&mut self, sc: Scene) {
        self.scenes.push(sc);
    }

    /// Return a reference to the scene currently in use.
    pub fn cur_scene(&self) -> &Scene {
        &self.scenes[self.cur]
    }

    /// Return a mutable reference to the scene currently in use.
    pub fn cur_scene_mut(&mut self) -> &mut Scene {
        &mut self.scenes[self.cur]
    }

    /// Change the currently used scene to the one at the provided index.
    pub fn change_scene(&mut self, new_idx: usize) {
        self.cur = new_idx;
    }

    /// Start displaying the UI elements into the current scene. Updates this window every key
    /// press. Exits with a user defined code when an element causes this to happen.
    pub fn run(&mut self) -> u32 {
        let mut handle = io::stdout();

        'full: loop {
            let scene = self.cur_scene_mut();
            scene.draw();

            for (y, row) in scene.win.data.iter().enumerate() {
                for (x, &ch) in row.iter().enumerate() {
                    let p = Point::new(x as i32, y as i32) + scene.win.top_left;
                    let _ = queue!(
                        handle,
                        cursor::MoveTo(p.x as u16, p.y as u16),
                        style::Print(ch)
                    );
                }
            }

            let _ = handle.flush();

            while let event::Event::Key(ke) = event::read().expect("what") {
                if ke.is_press() {
                    let action = match ke.code {
                        // Has arrow keys, wasd, and, for the vim users among us, hjkl.
                        event::KeyCode::Left
                        | event::KeyCode::Char('a')
                        | event::KeyCode::Char('h') => Nav::Move(Point::new(-1, 0)),
                        event::KeyCode::Right
                        | event::KeyCode::Char('d')
                        | event::KeyCode::Char('l') => Nav::Move(Point::new(1, 0)),
                        event::KeyCode::Down
                        | event::KeyCode::Char('s')
                        | event::KeyCode::Char('j') => Nav::Move(Point::new(0, 1)),
                        event::KeyCode::Up
                        | event::KeyCode::Char('w')
                        | event::KeyCode::Char('k') => Nav::Move(Point::new(0, -1)),
                        event::KeyCode::Enter | event::KeyCode::Char(' ') => Nav::Activate,
                        _ => Nav::Null,
                    };

                    match action {
                        Nav::Move(p) => {
                            scene.disp_cursor(p);
                        }
                        Nav::Activate => {
                            let ev = scene.elements.get_mut(&scene.cursor).expect("No ui elements to activate").activate();

                            match ev {
                                Event::Exit(code) => return code,
                                Event::ChangeScene(idx) => self.cur = idx,
                                Event::Broadcast(d1, d2) => {
                                    for elem in scene.elements.values_mut() {
                                        elem.receive(d1, d2);
                                    }
                                }
                                Event::Null => continue,
                            }
                        }
                        Nav::Null => continue,
                    }
                    continue 'full;
                }
            }
        }
    }
}
