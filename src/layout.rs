pub mod container;
mod render_trait;
pub mod widget;

use self::{
    container::{Container, Item, RcCon},
    widget::{widget_type::WidgetType, Widget},
};
use crate::Config;
use crate::{
    error::{ToDoError, ToDoRes},
    todo::ToDo,
    ui::HandleEvent,
};
use crossterm::event::KeyEvent;
pub use render_trait::Render;
use std::{
    rc::Rc,
    sync::{Arc, Mutex},
    {cell::RefCell, str::FromStr},
};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Direction::Horizontal, Direction::Vertical, Rect},
    Frame,
};

/// Represents the layout of the user interface.
///
/// The `Layout` struct defines the layout of the user interface for the todo-tui application. It
/// consists of a tree of containers and widgets, which are used to organize and display the various
/// components of the application.
pub struct Layout {
    root: Rc<RefCell<Container>>,
    actual: Rc<RefCell<Container>>,
}

impl Layout {
    /// Parse and convert a string value to a `Constraint`.
    ///
    /// # Parameters
    ///
    /// - `value`: A string slice representing the layout constraint.
    ///
    /// # Returns
    ///
    /// Returns a `ToDoRes` containing the converted `Constraint` or an error if parsing fails.
    fn value_from_string(value: &str) -> ToDoRes<Constraint> {
        if value.is_empty() {
            return Ok(Constraint::Percentage(50));
        }

        match value.find('%') {
            Some(i) => {
                if i + 1 < value.len() {
                    Err(ToDoError::ParseUnknownValue)
                } else {
                    Ok(Constraint::Percentage(value[..i].parse()?))
                }
            }
            None => Ok(Constraint::Length(value.parse()?)),
        }
    }

    /// Create a new `Layout` from a template string.
    ///
    /// This function parses a template string and creates a new `Layout` instance based on the
    /// specified template. The template string defines the layout of the user interface, including
    /// the arrangement of containers and widgets.
    ///
    /// # Parameters
    ///
    /// - `template`: A string containing the layout template.
    /// - `data`: An `Arc<Mutex<ToDo>>` representing the shared to-do data.
    ///
    /// # Returns
    ///
    /// A `ToDoRes<Self>` result containing the created `Layout` if successful, or an error if
    /// parsing fails.
    pub fn from_str(template: &str, data: Arc<Mutex<ToDo>>, config: &Config) -> ToDoRes<Self> {
        // Find first '[' and move start of template to it (start of first container)
        let index = match template.find('[') {
            Some(i) => i,
            None => return Err(ToDoError::ParseNotStart),
        };
        let template = &template[index + 1..];

        // Define separators
        const ITEM_SEPARATOR: char = ',';
        const ARG_SEPARATOR: char = ':';
        const START_CONTAINER: char = '[';
        const END_CONTAINER: char = ']';

        let mut string = String::new();
        let mut item = String::new();

        let mut container: Vec<(Direction, Constraint, Vec<Item>, Vec<Constraint>)> = Vec::new();
        container.push((
            Direction::Vertical,
            Constraint::Percentage(50),
            Vec::new(),
            Vec::new(),
        ));

        for ch in template.chars() {
            match ch {
                START_CONTAINER => {
                    let new_direction = match container.last().unwrap().0 {
                        Vertical => Horizontal,
                        Horizontal => Vertical,
                    };
                    container.push((
                        new_direction,
                        Constraint::Percentage(50),
                        Vec::new(),
                        Vec::new(),
                    ));

                    string.clear();
                }
                END_CONTAINER => {
                    let cont = container.pop().unwrap();
                    // End of the brackets stack, end cycle
                    if container.is_empty() {
                        let root = Container::new(cont.2, cont.3, cont.0, None);
                        let actual =
                            Container::select_widget(root.clone(), config.get_init_widget()).unwrap();
                        actual.borrow_mut().actual_mut()?.focus();
                        // if let IItem::Widget(w) = actual.borrow_mut().actual_item_mut() {
                        //     w.widget.focus();
                        // } TODO remove comment
                        return Ok(Layout { root, actual });
                    }
                    let c = Item::Container(Container::new(cont.2, cont.3, cont.0, None));
                    container.last_mut().unwrap().2.push(c);
                    container.last_mut().unwrap().3.push(cont.1);
                    string.clear();
                }
                ARG_SEPARATOR => {
                    item = string;
                    string = String::new();
                }
                ITEM_SEPARATOR => {
                    // Skip leading ITEM_SEPARATOR
                    if string.is_empty() {
                        continue;
                    }
                    if item.is_empty() {
                        item = string.to_lowercase();
                        string.clear();
                    } else {
                        item = item.to_lowercase();
                        string = string.to_lowercase();
                    }
                    match item.as_str() {
                        "direction" => match string.as_str() {
                            "" | "vertical" => {
                                let direction = &mut container.last_mut().unwrap().0;
                                *direction = Direction::Vertical;
                            }
                            "horizontal" => {
                                let direction = &mut container.last_mut().unwrap().0;
                                *direction = Direction::Horizontal;
                            }
                            _ => return Err(ToDoError::ParseInvalidDirection(string)),
                        },
                        "size" => {
                            container.last_mut().unwrap().1 = Self::value_from_string(&string)?;
                        }
                        _ => {
                            let widget_type = WidgetType::from_str(&item)?;
                            let cont = container.last_mut().unwrap();
                            cont.2.push(Item::Widget(Widget::new(
                                widget_type,
                                data.clone(),
                                config,
                            )?));
                            cont.3.push(Self::value_from_string(&string)?);
                        }
                    }
                    item.clear();
                    string.clear();
                }
                ' ' => {}
                '\n' => {}
                _ => string.push(ch),
            };
        }
        Err(ToDoError::ParseNotEnd)
    }

    /// Move the focus within the layout hierarchy.
    ///
    /// # Parameters
    ///
    /// - `container`: An `RcCon` representing the current container being focused.
    /// - `direction`: A reference to the `Direction` indicating the movement direction.
    /// - `f`: A function pointer that determines the action for moving the focus.
    ///
    /// # Returns
    ///
    /// Returns an `Option<RcCon>` containing the new focused container or `None` if no valid
    /// container is found in the specified direction.
    fn move_focus(
        container: RcCon,
        direction: &Direction,
        f: fn(RcCon) -> Option<RcCon>,
    ) -> Option<RcCon> {
        let move_to_parent = || {
            let mut c = container.borrow_mut();
            if let Some(parent) = &c.parent {
                return Layout::move_focus(parent.clone(), direction, f).map(|ret| {
                    c.unfocus();
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

    /// Change the focus within the layout.
    ///
    /// # Parameters
    ///
    /// - `next`: An `Option<RcCon>` representing the new container to focus.
    fn change_focus(&mut self, next: Option<RcCon>) {
        let next = match next {
            Some(s) => s,
            None => return,
        };
        self.actual.borrow_mut().unfocus();
        next.borrow_mut().focus();
        self.actual = next;
    }

    /// Move the focus to the left.
    ///
    /// This method moves the focus to the container or widget to the left of the currently focused
    /// element within the layout.
    pub fn left(&mut self) {
        self.change_focus(Self::move_focus(
            Rc::clone(&self.actual),
            &Horizontal,
            Container::previous_item,
        ));
    }

    /// Move the focus to the right.
    ///
    /// This method moves the focus to the container or widget to the right of the currently focused
    /// element within the layout.
    pub fn right(&mut self) {
        self.change_focus(Self::move_focus(
            Rc::clone(&self.actual),
            &Horizontal,
            Container::next_item,
        ));
    }

    /// Move the focus upwards.
    ///
    /// This method moves the focus to the container or widget above the currently focused element
    /// within the layout.
    pub fn up(&mut self) {
        self.change_focus(Self::move_focus(
            Rc::clone(&self.actual),
            &Vertical,
            Container::previous_item,
        ));
    }

    /// Move the focus downwards.
    ///
    /// This method moves the focus to the container or widget below the currently focused element
    /// within the layout.
    pub fn down(&mut self) {
        self.change_focus(Self::move_focus(
            Rc::clone(&self.actual),
            &Vertical,
            Container::next_item,
        ));
    }

    /// Handle a key event.
    ///
    /// This method is used to handle key events within the layout. It passes the key event to the
    /// currently focused widget or container for processing.
    ///
    /// # Parameters
    ///
    /// - `event`: A reference to the `KeyEvent` to be handled.
    pub fn handle_key(&self, event: &KeyEvent) -> bool {
        self.actual
            .borrow_mut()
            .actual_mut()
            .unwrap() // TODO remove
            .handle_key(&event.code)
    }

    pub fn get_active_widget(&self) -> WidgetType {
        self.actual.borrow().get_active_type()
    }
}

impl Render for Layout {
    fn render<B: Backend>(&self, f: &mut Frame<B>) {
        self.root.borrow().render(f);
    }

    fn unfocus(&mut self) {
        self.actual.borrow_mut().unfocus();
    }

    fn focus(&mut self) {
        self.actual.borrow_mut().focus();
    }

    fn update_chunk(&mut self, chunk: Rect) {
        self.root.borrow_mut().update_chunk(chunk);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_layout() -> Layout {
        let mock_layout = r#"
        [
            Direction: Horizontal,
            Size: 50%,
            [
                List: 50%,
                Preview,
            ],
            [ Direction: Vertical,
              Done,
              [ 
                Contexts,
                Projects,
              ],
            ],
        ]
        "#;
        Layout::from_str(
            mock_layout,
            Arc::new(Mutex::new(ToDo::default())),
            &Config::default(),
        )
        .unwrap()
    }

    #[test]
    fn test_basic_movement() -> ToDoRes<()> {
        let mut l = mock_layout();
        let check_type = |widget_type, l: &Layout| -> ToDoRes<()> {
            let active = l.get_active_widget();
            if active != widget_type {
                panic!("Active widget must be {:?} not {:?}.", widget_type, active)
            }
            Ok(())
        };

        check_type(WidgetType::List, &l)?;

        l.right();
        check_type(WidgetType::Done, &l)?;

        l.right();
        check_type(WidgetType::Done, &l)?;

        l.down();
        check_type(WidgetType::Context, &l)?;

        l.right();
        check_type(WidgetType::Project, &l)?;

        l.down();
        check_type(WidgetType::Project, &l)?;

        l.left();
        check_type(WidgetType::Context, &l)?;

        l.left();
        check_type(WidgetType::List, &l)?;

        l.right();
        check_type(WidgetType::Context, &l)?;

        l.left();
        check_type(WidgetType::List, &l)?;

        l.up();
        check_type(WidgetType::List, &l)?;

        Ok(())
    }

    #[test]
    fn test_from_string() -> ToDoRes<()> {
        let str_layout = r#"
            [
              Direction:Horizontal,
              Size: 50%,
              List: 50%,
              [ dIrEcTiOn: VeRtIcAl,
                Done,
                Hashtags: 50%,
              ],
              Projects: 50%,
            ]
            
            Direction: ERROR,
        "#;

        Layout::from_str(str_layout, Arc::new(Mutex::new(ToDo::default())), &Config::default())?;
        Ok(())
    }
}
