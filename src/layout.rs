pub mod container;
mod render_trait;
pub mod widget;

use container::Container;
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::Mutex;
use widget::{widget_type::WidgetType, Widget};

use crate::Config;
use crate::{
    error::{ToDoError, ToDoRes},
    todo::ToDo,
    ui::HandleEvent,
};
use crossterm::event::KeyEvent;

pub use render_trait::Render;

use std::str::FromStr;
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
#[derive(Debug)]
pub struct Layout {
    containers: Vec<Container>,
    act: usize,
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
                    println!("Error: {}", value);
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
        log::debug!("Layout from str: {}", template);
        println!("{}", template);

        // Define separators
        const ITEM_SEPARATOR: char = ',';
        const ARG_SEPARATOR: char = ':';
        const START_CONTAINER: char = '[';
        const END_CONTAINER: char = ']';

        let mut string = String::new();
        let mut item = String::new();

        let mut constraints_stack: Vec<Vec<Constraint>> = Vec::new();
        constraints_stack.push(Vec::new());
        let mut containers: Vec<Container> = Vec::new();
        let mut layout = Layout {
            act: Container::add_container(&mut containers, Container::default()),
            containers,
        };

        for ch in template.chars() {
            match ch {
                START_CONTAINER => {
                    println!("ERROR: |{}|{}|", string, item);
                    string.clear();
                    let mut cont = Container::default();
                    cont.parent = Some(layout.act);
                    cont.set_direction(match layout.act().get_direction() {
                        Direction::Horizontal => Direction::Vertical,
                        Direction::Vertical => Direction::Horizontal,
                    });
                    layout.act = Container::add_container(&mut layout.containers, cont);
                    constraints_stack.push(Vec::new());
                }
                END_CONTAINER => {
                    println!("Act: {}, Constraints: {:?}", layout.act, constraints_stack.last());
                    layout
                        .act_mut()
                        .set_constraints(constraints_stack.pop().unwrap());
                    layout.act = match layout.act().parent {
                        Some(parent) => parent,
                        // We are at root. Return created layout.
                        None => {
                            Container::actualize_layout(&mut layout);
                            return Ok(layout);
                        }
                    };
                    // constraints_stack.pop() = Vec::new();
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
                    // let x: Vec<&str> = string.splitn(2,':').map(|s| s.trim()).collect();
                    // println!("String: {}, Item: {}", string, item);
                    if item.is_empty() {
                        item = string.to_lowercase();
                        string.clear();
                    } else {
                        item = item.to_lowercase();
                        string = string.to_lowercase();
                    }
                    match item.as_str() {
                        "direction" => match string.as_str() {
                            "" | "vertical" => layout.act_mut().set_direction(Direction::Vertical),
                            "horizontal" => layout.act_mut().set_direction(Direction::Horizontal),
                            _ => return Err(ToDoError::ParseInvalidDirection(string)),
                        },
                        "size" => {
                            println!("size: {}", string);
                            constraints_stack
                                .last_mut()
                                .unwrap()
                                .push(Self::value_from_string(&string)?);
                        }
                        _ => {
                            let widget_type = WidgetType::from_str(&item)?;
                            layout.act_mut().add_widget(Widget::new(
                                widget_type,
                                data.clone(),
                                config,
                            )?);
                            constraints_stack
                                .last_mut()
                                .unwrap()
                                .push(Self::value_from_string(&string)?);
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

    fn act(&self) -> &Container {
        &self.containers[self.act]
    }

    fn act_mut(&mut self) -> &mut Container {
        &mut self.containers[self.act]
    }

    /// Change the focus within the layout.
    ///
    /// # Parameters
    ///
    /// - `next`: An `Option<RcCon>` representing the new container to focus.
    fn change_focus(&mut self, direction: Direction, f: impl Fn(&mut Container) -> bool) -> bool {
        let old_act = self.act;
        while *self.act().get_direction() != direction {
            match self.act().parent {
                Some(index) => self.act = index,
                None => return false,
            }
        }
        if f(self.act_mut()) {
            Container::actualize_layout(self);
            true
        } else {
            match self.act().parent {
                // check if there is upper container that can handle change
                Some(index) => {
                    self.act = index;
                    if self.change_focus(direction, f) {
                        true
                    } else {
                        self.act = old_act;
                        false
                    }
                }
                None => {
                    // Do not move from starting position if you can't.
                    self.act = old_act;
                    false
                }
            }
        }
    }

    /// Move the focus to the left.
    ///
    /// This method moves the focus to the container or widget to the left of the currently focused
    /// element within the layout.
    pub fn left(&mut self) -> bool {
        self.change_focus(Horizontal, Container::previous_item)
    }

    /// Move the focus to the right.
    ///
    /// This method moves the focus to the container or widget to the right of the currently focused
    /// element within the layout.
    pub fn right(&mut self) -> bool {
        self.change_focus(Horizontal, Container::next_item)
    }

    /// Move the focus upwards.
    ///
    /// This method moves the focus to the container or widget above the currently focused element
    /// within the layout.
    pub fn up(&mut self) -> bool {
        self.change_focus(Vertical, Container::previous_item)
    }

    /// Move the focus downwards.
    ///
    /// This method moves the focus to the container or widget below the currently focused element
    /// within the layout.
    pub fn down(&mut self) -> bool {
        self.change_focus(Vertical, Container::next_item)
    }

    /// Handle a key event.
    ///
    /// This method is used to handle key events within the layout. It passes the key event to the
    /// currently focused widget or container for processing.
    ///
    /// # Parameters
    ///
    /// - `event`: A reference to the `KeyEvent` to be handled.
    pub fn handle_key(&mut self, event: &KeyEvent) -> bool {
        match self.act_mut().actual_mut() {
            Some(widget) => widget.handle_key(&event.code),
            None => panic!("Actual is not widget"),
        }
    }

    pub fn get_active_widget(&self) -> WidgetType {
        match self.act().get_active_type() {
            Some(widget_type) => widget_type,
            None => panic!("Actual is not widget"),
        }
    }
}

impl Render for Layout {
    fn render<B: Backend>(&self, f: &mut Frame<B>) {
        self.act().render(f, &self.containers);
    }

    fn unfocus(&mut self) {
        match self.act_mut().actual_mut() {
            Some(w) => w.unfocus(),
            None => panic!("Actual to unfocus is  not a widget"),
        }
    }

    fn focus(&mut self) {
        match self.act_mut().actual_mut() {
            Some(w) => w.focus(),
            None => panic!("Actual to focus is not a widget"),
        }
    }

    fn update_chunk(&mut self, chunk: Rect) {
        Container::update_chunk(chunk, &mut self.containers, 0);
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
        assert_eq!(l.get_active_widget(), WidgetType::List);

        assert!(l.right());
        assert_eq!(l.get_active_widget(), WidgetType::Done);
        assert!(l.left());
        assert_eq!(l.get_active_widget(), WidgetType::List);
        assert!(l.right());
        assert_eq!(l.get_active_widget(), WidgetType::Done);
        assert!(!l.right());
        assert_eq!(l.get_active_widget(), WidgetType::Done);
        assert!(l.down());
        assert_eq!(l.get_active_widget(), WidgetType::Context);
        assert!(l.right());
        assert_eq!(l.get_active_widget(), WidgetType::Project);
        assert!(!l.down());
        assert_eq!(l.get_active_widget(), WidgetType::Project);
        assert!(l.left());
        assert_eq!(l.get_active_widget(), WidgetType::Context);
        assert!(l.left());
        assert_eq!(l.get_active_widget(), WidgetType::List);
        assert!(l.right());
        assert_eq!(l.get_active_widget(), WidgetType::Context);
        assert!(l.left());
        assert_eq!(l.get_active_widget(), WidgetType::List);
        assert!(!l.up());
        assert_eq!(l.get_active_widget(), WidgetType::List);

        Ok(())
    }

    #[test]
    fn test_from_string() -> ToDoRes<()> {
        let str_layout = r#"
            [
              dIrEcTiOn:HoRiZoNtAl,
              Size: 50%,
              List: 50%,
              [
                Done,
                Hashtags: 50%,
              ],
              Projects: 50%,
            ]
            
            Direction: ERROR,
        "#;

        let mut layout = Layout::from_str(
            str_layout,
            Arc::new(Mutex::new(ToDo::default())),
            &Config::default(),
        )?;
        assert_eq!(layout.containers.len(), 2);

        assert_eq!(*layout.containers[0].get_direction(), Horizontal);
        assert_eq!(layout.containers[0].parent, None);
        while layout.containers[0].previous_item() {}
        assert_eq!(
            layout.containers[0].get_active_type(),
            Some(WidgetType::List)
        );
        assert!(layout.containers[0].next_item());
        assert_eq!(layout.containers[0].get_active_type(), None);
        assert!(layout.containers[0].next_item());
        assert_eq!(
            layout.containers[0].get_active_type(),
            Some(WidgetType::Project)
        );
        assert!(!layout.containers[0].next_item());

        assert_eq!(*layout.containers[1].get_direction(), Vertical);
        assert_eq!(layout.containers[1].parent, Some(0));
        while layout.containers[1].previous_item() {}
        assert_eq!(
            layout.containers[1].get_active_type(),
            Some(WidgetType::Done)
        );
        assert!(layout.containers[1].next_item());
        assert_eq!(
            layout.containers[1].get_active_type(),
            Some(WidgetType::Hashtag)
        );
        assert!(!layout.containers[1].next_item());

        Ok(())
    }
}
