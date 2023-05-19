mod container;
mod widget;

use std::cell::RefCell;
use std::rc::Rc;

use crate::error::ErrorToDo;

use self::container::InitItem;
use container::Container;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Rect},
    Frame,
};
use widget::{Widget, WidgetType};

#[allow(dead_code)]
pub struct Layout {
    root: Rc<RefCell<Container>>,
    active: WidgetType,
    actual: Rc<RefCell<Container>>,
}

#[allow(dead_code)]
impl Layout {
    pub fn new(chunk: Rect) -> Layout {
        let input_widget = Widget::new(WidgetType::Input, "Input");
        let list_widget = Widget::new(WidgetType::List, "List");
        let done_widget = Widget::new(WidgetType::Done, "Done");
        let categories_widget = Widget::new(WidgetType::Categories, "Categories");

        let root = Container::new(
            vec![
                InitItem::Widget(input_widget),
                InitItem::Container(Container::new(
                    vec![
                        InitItem::Widget(list_widget),
                        InitItem::Container(Container::new(
                            vec![
                                InitItem::Widget(done_widget),
                                InitItem::Widget(categories_widget),
                            ],
                            vec![Constraint::Percentage(50), Constraint::Percentage(50)],
                            Direction::Vertical,
                            None,
                        )),
                    ],
                    vec![Constraint::Percentage(50), Constraint::Percentage(50)],
                    Direction::Horizontal,
                    None,
                )),
            ],
            vec![Constraint::Length(3), Constraint::Percentage(30)],
            Direction::Vertical,
            None,
        );
        let actual = Container::select_widget(&root, &WidgetType::List).unwrap(); // TODO
        root.borrow_mut().update_chunks(chunk);

        Layout {
            root,
            active: WidgetType::List,
            actual,
        }
    }

    pub fn move_focus(
        &mut self,
        direction: Direction,
        f: fn(&Rc<RefCell<Container>>) -> Option<Rc<RefCell<Container>>>,
    ) {
        if self.actual.borrow().direction == direction {
            match f(&self.actual) {
                Some(actual) => self.actual = actual,
                None => {}
            }
        }
    }

    pub fn left(&mut self) {
        self.move_focus(Direction::Horizontal, Container::previous_item);
    }

    pub fn right(&mut self) {
        self.move_focus(Direction::Horizontal, Container::next_item);
    }

    pub fn up(&mut self) {
        self.move_focus(Direction::Vertical, Container::previous_item);
    }

    pub fn down(&mut self) {
        self.move_focus(Direction::Vertical, Container::next_item);
    }

    pub fn select_widget(&mut self, widget_type: &WidgetType) -> Result<(), ErrorToDo> {
        self.actual = Container::select_widget(&self.root, widget_type)?;
        Ok(())
    }

    pub fn update_chunks(&mut self, chunk: Rect) {
        self.root.borrow_mut().update_chunks(chunk);
    }

    pub fn render<B>(&self, f: &mut Frame<B>)
    where
        B: Backend,
    {
        self.root.as_ref().borrow().render_recursive(f);
    }
}
