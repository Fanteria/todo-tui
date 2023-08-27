use crate::{CONFIG, ui::EventHandler};
use crate::ui::{UIEvent, HandleEvent};
use crossterm::event::KeyCode;
use tui::widgets::ListState;

pub struct WidgetList {
    state: ListState,
    first: usize,
    size: usize,
    shift: usize,
    event_handler: EventHandler,
}

impl WidgetList {
    pub fn act(&self) -> usize {
        self.state.selected().unwrap_or(0)
    }

    pub fn index(&self) -> usize {
        self.act() + self.first
    }

    pub fn state(&self) -> ListState {
        self.state.clone()
    }

    pub fn set_shift(&mut self, shift: usize) {
        self.shift = shift
    }

    pub fn set_size(&mut self, size: u16) {
        self.size = size as usize
    }

    pub fn down(&mut self, len: usize) {
        let act = self.act();
        log::trace!("List go down: act: {}, len: {}", act, len);
        if len <= self.size {
            if len > act + 1 {
                self.state.select(Some(act + 1));
            }
        } else if self.size <= act + 1 + CONFIG.list_shift {
            if self.first + self.size + 1 < len {
                self.first += 1;
            } else if self.size > act + 1 {
                self.state.select(Some(act + 1));
            }
        } else {
            self.state.select(Some(act + 1));
        }
    }

    pub fn up(&mut self) {
        let act = self.act();
        log::trace!("List go up: act: {}", act);
        if act <= CONFIG.list_shift {
            if self.first > 0 {
                self.first -= 1;
            } else if act > 0 {
                self.state.select(Some(act - 1));
            }
        } else {
            self.state.select(Some(act - 1));
        }
    }

    /// (old, new)
    pub fn next(&mut self, len: usize) -> Option<(usize, usize)> {
        if (len <= self.size) && len <= self.act() + 1 {
            None
        } else {
            let old = self.index();
            self.down(len);
            Some((old, self.index()))
        }
    }

    pub fn prev(&mut self) -> Option<(usize, usize)> {
        if self.act() == 0 {
            None
        } else {
            let old = self.index();
            self.up();
            Some((old, self.index()))
        }
    }

    pub fn first(&mut self) {
        self.state.select(Some(0));
        self.first = 0;
    }

    pub fn last(&mut self, len: usize) {
        let shown_items = len - 1;
        if self.size > shown_items {
            self.first = 0;
            self.state.select(Some(shown_items));
        } else {
            self.first = shown_items - self.size;
            self.state.select(Some(self.size - 1));
        }
    }

    /// (first, last)
    pub fn range(&self) -> (usize, usize) {
        (self.first, self.first + self.size)
    }
}

impl Default for WidgetList {
    fn default() -> Self {
        let mut def = Self {
            state: ListState::default(),
            first: 0,
            size: 24,
            shift: 0,
            event_handler: CONFIG.list_keybind.clone(),
        };
        def.state.select(Some(0));
        def
    }
}

impl HandleEvent for WidgetList {
    fn get_event(&self, key: &KeyCode) -> UIEvent {
        self.event_handler.get_event(key)
    }

    fn handle_event(&mut self, event: UIEvent) -> bool {
        match event {
            UIEvent::ListDown => self.down(50),
            UIEvent::ListUp => self.up(),
            UIEvent::ListFirst => self.first(),
            UIEvent::ListLast => self.last(50),
            _ => return false,
        }
        true
    }
}
