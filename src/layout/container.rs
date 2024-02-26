mod holder;
mod item;

use self::holder::Holder;
use super::render_trait::Render;
use super::widget::{widget_type::WidgetType, Widget};
use crate::error::{ToDoError, ToDoRes};
use item::IItem;
pub use item::Item;
use std::{cell::RefCell, rc::Rc};
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

pub type RcCon = Rc<RefCell<Container>>;

/// Represents a container that can hold widgets and other containers.
///
/// A `Container` is a component that can hold a collection of `Item`s, which can be either
/// widgets or nested containers. It provides methods for rendering, focusing, and updating
/// the contained items.
pub struct Container {
    items: Vec<IItem>,
    layout: Layout,
    pub direction: Direction,
    pub parent: Option<RcCon>,
    act_index: usize,
}

impl Container {
    /// Creates a new container with the given items, constraints, direction, and parent.
    ///
    /// # Parameters
    ///
    /// - `items`: A vector of items (widgets or containers) to be placed within the container.
    /// - `constraints`: A vector of constraints defining how items should be laid out within
    ///   the container.
    /// - `direction`: The layout direction of the container (Vertical or Horizontal).
    /// - `parent`: An optional reference to the parent container, if this container is nested
    ///   within another container.
    ///
    /// # Returns
    ///
    /// A reference-counted (`Rc`) reference to the newly created `Container`.
    pub fn new(
        items: Vec<Item>,
        constraints: Vec<Constraint>,
        direction: Direction,
        parent: Option<RcCon>,
    ) -> RcCon {
        let container = Rc::new(RefCell::new(Container {
            items: Vec::new(),
            layout: Layout::default()
                .direction(direction.clone())
                .constraints(constraints),
            direction,
            parent,
            act_index: 0,
        }));

        items.into_iter().for_each(|item| {
            container
                .borrow_mut()
                .items
                .push(IItem::new(item, container.clone()))
        });
        container
    }

    /// Returns focused item as reference.
    fn actual_item(&self) -> &IItem {
        &self.items[self.act_index]
    }

    /// Returns focused item as mutable reference.
    fn actual_item_mut(&mut self) -> &mut IItem {
        &mut self.items[self.act_index]
    }

    /// Returns a reference to the currently active item within the container.
    ///
    /// # Returns
    ///
    /// A result containing a reference to the active `Widget` or an error if the active item
    /// is not a widget.
    #[allow(dead_code)]
    pub fn actual(&self) -> ToDoRes<&Widget> {
        self.actual_item().actual()
    }

    /// Returns a mutable reference to the currently active item within the container.
    ///
    /// # Returns
    ///
    /// A result containing a mutable reference to the active `Widget` or an error if the active
    /// item is not a widget.
    pub fn actual_mut(&mut self) -> ToDoRes<&mut Widget> {
        self.actual_item_mut().actual_mut()
    }

    /// Updates the currently active item within the container.
    /// This function recursively searches for the next active item based on the container's layout.
    fn update_actual(container: &RcCon) -> RcCon {
        let mut borrow = container.borrow_mut();
        match borrow.actual_item() {
            IItem::Widget(_) => {
                borrow.focus();
                Rc::clone(container)
            }
            IItem::Container(cont) => Container::update_actual(cont),
        }
    }

    /// Attempts to change the active item within the container based on a condition.
    ///
    /// # Parameters
    ///
    /// - `container`: A reference-counted (Rc) reference to the container to navigate within.
    /// - `condition`: A function that takes a reference to the current container and returns
    ///   `true` if the condition to change the active item is met, or `false` otherwise.
    /// - `change`: A function that takes a mutable reference to the current container and
    ///   performs the necessary changes to the active item.
    ///
    /// # Returns
    ///
    /// An option containing either an updated reference to the container with the active item
    /// changed, or `None` if the condition is not met.
    fn change_item(
        container: &RcCon,
        condition: fn(&Container) -> bool,
        change: fn(&mut Container),
    ) -> Option<RcCon> {
        if condition(&container.borrow()) {
            return None;
        }
        container.borrow_mut().unfocus();
        change(&mut container.borrow_mut());
        container.borrow_mut().focus();
        Some(Container::update_actual(container))
    }

    /// Attempts to select the next item within the container.
    ///
    /// # Parameters
    ///
    /// - `container`: A reference-counted (Rc) reference to the container to navigate within.
    ///
    /// # Returns
    ///
    /// An option containing either an updated reference to the container with the next item
    /// as the active item, or `None` if there is no next item to select within the container.
    pub fn next_item(container: RcCon) -> Option<RcCon> {
        Container::change_item(
            &container,
            |c| c.act_index + 1 >= c.items.len(),
            |c| c.act_index += 1,
        )
    }

    /// Attempts to select the previous item within the container.
    ///
    /// # Parameters
    ///
    /// - `container`: A reference-counted (Rc) reference to the container to navigate within.
    ///
    /// # Returns
    ///
    /// An option containing either an updated reference to the container with the previous item
    /// as the active item, or `None` if there is no previous item to select within the container.
    ///
    pub fn previous_item(container: RcCon) -> Option<RcCon> {
        Container::change_item(&container, |c| c.act_index == 0, |c| c.act_index -= 1)
    }

    /// Finds and selects a specific widget type within the container.
    ///
    /// # Parameters
    ///
    /// - `container`: A reference-counted (Rc) reference to the container to search within.
    /// - `widget_type`: The `WidgetType` enum variant representing the target widget type.
    ///
    /// # Returns
    ///
    /// A result containing either an updated reference to the container with the selected widget
    /// type as the active item, or an error if the widget type is not found within the container.
    pub fn select_widget(container: RcCon, widget_type: WidgetType) -> ToDoRes<RcCon> {
        let mut borrowed = container.borrow_mut();
        for (index, item) in borrowed.items.iter().enumerate() {
            match item {
                IItem::Widget(w) => {
                    if w.widget_type() == widget_type {
                        borrowed.act_index = index;
                        return Ok(container.clone());
                    }
                }
                IItem::Container(container) => {
                    let cont = Container::select_widget(container.clone(), widget_type);
                    if cont.is_ok() {
                        borrowed.act_index = index;
                        return cont;
                    }
                }
            }
        }
        Err(ToDoError::WidgetDoesNotExist)
    }

    pub fn get_active_type(&self) -> WidgetType {
        if let IItem::Widget(w) = self.actual_item() {
            return w.data.widget_type();
        };
        panic!("The current item is expected to be a widget.");
    }

}

impl Render for Container {
    fn render<B: Backend>(&self, f: &mut Frame<B>) {
        self.items.iter().for_each(|i| i.render(f));
    }

    fn focus(&mut self) {
        self.actual_item_mut().focus();
    }

    fn unfocus(&mut self) {
        self.actual_item_mut().unfocus();
    }

    fn update_chunk(&mut self, chunk: Rect) {
        let chunks = self.layout.split(chunk);
        self.items
            .iter_mut()
            .enumerate()
            .for_each(|(i, item)| item.update_chunk(chunks[i]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, todo::ToDo};
    use std::sync::{Arc, Mutex};
    use tui::layout::Direction::{Horizontal, Vertical};
    use WidgetType::*;

    fn create_testing_container() -> RcCon {
        let todo = Arc::new(Mutex::new(ToDo::default()));
        let list_widget = Widget::new(WidgetType::List, todo.clone(), &Config::default()).unwrap();
        let done_widget = Widget::new(WidgetType::Done, todo.clone(), &Config::default()).unwrap();
        let project_widget = Widget::new(WidgetType::Project, todo, &Config::default()).unwrap();
        Container::new(
            vec![Item::Container(Container::new(
                vec![
                    Item::Widget(list_widget),
                    Item::Container(Container::new(
                        vec![Item::Widget(done_widget), Item::Widget(project_widget)],
                        vec![Constraint::Percentage(50), Constraint::Percentage(50)],
                        Vertical,
                        None,
                    )),
                ],
                vec![Constraint::Percentage(50), Constraint::Percentage(50)],
                Horizontal,
                None,
            ))],
            vec![Constraint::Length(3), Constraint::Percentage(30)],
            Vertical,
            None,
        )
    }

    fn check_active_test(container: &RcCon, widget_type: WidgetType) {
        let active = container.borrow().get_active_type();
        if active != widget_type {
            panic!("Active widget must be {:?} not {:?}.", widget_type, active)
        }
    }

    #[test]
    fn test_selecting_widget() {
        let c = create_testing_container();
        let check = |widget_type| match Container::select_widget(c.clone(), widget_type) {
            Ok(c) => {
                check_active_test(&c, widget_type);
                Ok(())
            }
            Err(e) => Err(e),
        };

        check(List).unwrap();
        check(Done).unwrap();
        check(Project).unwrap();
        assert!(
            check(Context).is_err(),
            "Widget with type Context is not in container."
        );
    }

    #[test]
    fn test_next_item() -> ToDoRes<()> {
        let c = create_testing_container();

        // Test next widget in child container.
        let actual = Container::select_widget(c.clone(), List)?;
        let next = Container::next_item(actual).unwrap();
        check_active_test(&next, Done);

        // Test next widget in same container.
        let actual = Container::select_widget(c.clone(), Done)?;
        let next = Container::next_item(actual).unwrap();
        check_active_test(&next, Project);

        // Test next in container have not default value
        let actual = Container::select_widget(c, List)?;
        let next = Container::next_item(actual.clone()).unwrap();
        check_active_test(&next, Project);

        // Test return value if there is no next item
        assert!(Container::next_item(actual.clone()).is_none());
        assert!(Container::next_item(actual.clone()).is_none());
        assert!(Container::next_item(actual.clone()).is_none());
        assert_eq!(actual.borrow().act_index, 1);
        check_active_test(&next, Project);

        Ok(())
    }

    #[test]
    fn test_previous_item() -> ToDoRes<()> {
        let c = create_testing_container();

        // Test previous widget in same container.
        let actual = Container::select_widget(c, Project)?;
        let prev = Container::previous_item(actual).unwrap();

        // Test return value if there is no previous item
        assert!(Container::previous_item(prev.clone()).is_none());
        assert!(Container::previous_item(prev.clone()).is_none());
        assert!(Container::previous_item(prev.clone()).is_none());
        assert_eq!(prev.borrow().act_index, 0);
        check_active_test(&prev, Done);

        Ok(())
    }
}
