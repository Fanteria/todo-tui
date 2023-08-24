use super::{widget_list::WidgetList, widget_state::RCToDo, widget_trait::State, Widget};
use crate::{
    todo::{ToDo, ToDoData},
    utils::get_block,
};
use crossterm::event::{KeyCode, KeyEvent};
use tui::{backend::Backend, style::Style, widgets::List, Frame};

pub struct StateList {
    state: WidgetList,
    style: Style,
    data_type: ToDoData,
    data: RCToDo,
    focus: bool,
}

impl StateList {
    pub fn new(data_type: ToDoData, data: RCToDo, style: Style) -> Self {
        Self {
            state: WidgetList::default(),
            style,
            data_type,
            data,
            focus: false,
        }
    }

    pub fn len(&self) -> usize {
        self.data.lock().unwrap().len(self.data_type)
    }

    fn swap_tasks(&mut self, first: usize, second: usize) {
        log::trace!("Swap tasks with indexes: {}, {}", first, second);
        self.data
            .lock()
            .unwrap()
            .swap_tasks(self.data_type, first, second);
    }

    fn move_task(&mut self, r#move: fn(&mut ToDo, ToDoData, usize)) {
        let index = self.state.index();
        log::info!("Remove task with index {index}.");
        r#move(&mut self.data.lock().unwrap(), self.data_type, index);
        let len = self.len();
        if len <= index && len > 0 {
            self.state.up();
            // self.state.select(Some(len - 1));
        }
    }
}

impl State for StateList {
    fn handle_key(&mut self, event: &KeyEvent) {
        if self.len() == 0 {
            return;
        }
        if !self.state.handle_key(event, self.len()) {
            match event.code {
                KeyCode::Char('U') => {
                    if let Some((first, second)) = self.state.prev() {
                        self.swap_tasks(first, second)
                    }
                }
                KeyCode::Char('D') => {
                    if let Some((first, second)) = self.state.next(self.len()) {
                        self.swap_tasks(first, second)
                    }
                }
                KeyCode::Char('x') => self.move_task(ToDo::remove_task),
                KeyCode::Char('d') => self.move_task(ToDo::move_task),
                KeyCode::Enter => {
                    self.data
                        .lock()
                        .unwrap()
                        .set_active(self.data_type, self.state.index());
                }
                _ => {}
            }
        }
    }

    fn render<B: Backend>(&self, f: &mut Frame<B>, _: bool, widget: &Widget) {
        let data = self.data.lock().unwrap();
        let filtered = data.get_filtered(self.data_type);
        let (first, last) = self.state.range();
        let filtered = filtered.slice(first, last);
        let list = List::new(filtered).block(get_block(&widget.title, self.focus));
        if !self.focus {
            f.render_widget(list, widget.chunk)
        } else {
            let list = list.highlight_style(self.style);
            f.render_stateful_widget(list, widget.chunk, &mut self.state.state());
        }
    }

    fn focus(&mut self) {
        self.focus = true;
        let len = self.len();
        if self.state.act() >= len && len > 0 {
            self.state.last()
        }
    }

    fn unfocus(&mut self) {
        self.focus = false;
    }
}
