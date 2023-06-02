pub mod container;
pub mod widget;
mod widget_state;
pub mod widget_type;

use self::{
    container::{Container, InitItem, Item},
    widget::Widget,
    widget_type::WidgetType,
};
use crate::{error::ErrorToDo, todo::ToDo};
use std::cell::RefCell;
use std::rc::Rc;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Direction::Horizontal, Direction::Vertical, Rect},
    // widgets::Widget,
    Frame,
};

type RcCon = Rc<RefCell<Container>>;

pub struct Layout {
    root: Rc<RefCell<Container>>,
    actual: Rc<RefCell<Container>>,
}

impl Layout {
    pub fn new(chunk: Rect, actual: WidgetType, data: Rc<ToDo>) -> Layout {
        let input_widget = Widget::new(WidgetType::Input, "Input", data.clone());
        let list_widget = Widget::new(WidgetType::List, "List", data.clone());
        let done_widget = Widget::new(WidgetType::Done, "Done", data.clone());
        let categories_widget = Widget::new(WidgetType::Project, "Projects", data.clone());

        let root = Container::new(
            vec![
                InitItem::InitWidget(input_widget),
                InitItem::InitContainer(Container::new(
                    vec![
                        InitItem::InitWidget(list_widget),
                        InitItem::InitContainer(Container::new(
                            vec![
                                InitItem::InitWidget(done_widget),
                                InitItem::InitWidget(categories_widget),
                            ],
                            vec![Constraint::Percentage(50), Constraint::Percentage(50)],
                            Vertical,
                            None,
                        )),
                    ],
                    vec![Constraint::Percentage(50), Constraint::Percentage(50)],
                    Horizontal,
                    None,
                )),
            ],
            vec![Constraint::Length(3), Constraint::Percentage(30)],
            Vertical,
            None,
        );
        let actual = Container::select_widget(root.clone(), actual).unwrap(); // TODO
        root.borrow_mut().update_chunks(chunk);

        Layout { root, actual }
    }

    pub fn move_focus(
        container: RcCon,
        direction: &Direction,
        f: fn(RcCon) -> Option<RcCon>,
    ) -> Option<RcCon> {
        let move_to_parent = || {
            let mut c = container.borrow_mut();
            if let Some(parent) = &c.parent {
                return Layout::move_focus(parent.clone(), direction, f).map(|ret| {
                    c.active = false;
                    ret
                });
            }
            None
        };

        if container.borrow().direction == *direction {
            return f(container.clone()).or_else(move_to_parent);
        }

        move_to_parent()
    }

    pub fn left(&mut self) {
        let left = Layout::move_focus(
            Rc::clone(&self.actual),
            &Horizontal,
            Container::previous_item,
        );
        if let Some(actual) = left {
            self.actual = actual;
        }
    }

    pub fn right(&mut self) {
        let right = Layout::move_focus(Rc::clone(&self.actual), &Horizontal, Container::next_item);
        if let Some(actual) = right {
            self.actual = actual;
        }
    }

    pub fn up(&mut self) {
        let up = Layout::move_focus(Rc::clone(&self.actual), &Vertical, Container::previous_item);
        if let Some(actual) = up {
            self.actual = actual;
        }
    }

    pub fn down(&mut self) {
        let down = Layout::move_focus(Rc::clone(&self.actual), &Vertical, Container::next_item);
        if let Some(actual) = down {
            self.actual = actual;
        }
    }

    #[allow(dead_code)]
    pub fn select_widget(&mut self, widget_type: WidgetType) -> Result<(), ErrorToDo> {
        self.actual = Container::select_widget(self.root.clone(), widget_type)?;
        Ok(())
    }

    pub fn active_widget(&self) -> Option<&mut Widget> {
        let x = self.actual.borrow_mut().actual_item(); 
        match x {
            Item::Widget(widget) => Some(&mut widget.widget),
            Item::Container(_) => None,
        }

        // match self.actual.borrow_mut().actual_item() {
        //     Item::Widget(widget) => Some(widget.widget),
        //     Item::Container(_) => None,
        // }
    }

    pub fn update_chunks(&mut self, chunk: Rect) {
        self.root.borrow_mut().update_chunks(chunk);
    }

    pub fn render<B>(&self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        self.root.borrow().render_recursive(f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_layout() -> Layout {
        Layout::new(
            Rect::new(0, 0, 0, 0),
            WidgetType::List,
            Rc::new(ToDo::new(false)),
        )
    }

    #[test]
    fn test_select_widget() {}

    #[test]
    fn test_active_widget() {}

    #[test]
    fn test_basic_movement() {
        let layout = mock_layout();

        // assert_eq!(
        //     layout.active_widget().unwrap().widget_type,
        //     WidgetType::List
        // );
    }
}
