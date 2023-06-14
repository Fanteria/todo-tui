use super::{state_input::StateInput, state_list::StateList, widget_type::WidgetType};
use crate::todo::ToDo;
use std::cell::RefCell;
use std::rc::Rc;
use tui::widgets::ListItem;

type RCToDo = Rc<RefCell<ToDo>>;

#[enum_dispatch(State)]
pub enum WidgetState {
    Input(StateInput),
    List(StateList),
}

impl WidgetState {
    pub fn new(widget_type: &WidgetType, data: Rc<RefCell<ToDo>>) -> Self {
        match widget_type {
            WidgetType::Input => WidgetState::Input(StateInput::new(data)),
            WidgetType::List => WidgetState::List(StateList::new(
                |todo| Into::<Vec<ListItem>>::into(todo.get_pending_filtered()),
                data,
            )),
            WidgetType::Done => WidgetState::List(StateList::new(
                |todo| Into::<Vec<ListItem>>::into(todo.get_done_filtered()),
                data,
            )),
            WidgetType::Project => WidgetState::List(StateList::new(
                |todo| Into::<Vec<ListItem>>::into(todo.get_projects()),
                data,
            )),
            WidgetType::Context => WidgetState::List(StateList::new(
                |todo| Into::<Vec<ListItem>>::into(todo.get_contexts()),
                data,
            )),
        }
    }
}
