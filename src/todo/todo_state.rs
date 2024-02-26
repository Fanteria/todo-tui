use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use todo_txt::Task;

use crate::config::ToDoConfig;

use super::{task_list::TaskSort, ToDo};

/// Enum to represent the state of ToDo data (pending or done).
#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum ToDoData {
    Pending,
    Done,
}

impl ToDoData {
    /// Gets a reference to the specified ToDo data.
    ///
    /// # Arguments
    ///
    /// * `data` - The type of ToDo data to retrieve.
    pub fn get_data<'a>(&self, todo: &'a ToDo) -> &'a Vec<Task> {
        match self {
            Self::Pending => &todo.pending,
            Self::Done => &todo.done,
        }
    }

    /// Gets a mutable reference to the specified ToDo data (pending or done).
    ///
    /// # Arguments
    ///
    /// * `data` - The type of ToDo data to retrieve (Pending or Done).
    pub fn get_data_mut<'a>(&self, todo: &'a mut ToDo) -> &'a mut Vec<Task> {
        todo.version += 1;
        match self {
            Self::Pending => &mut todo.pending,
            Self::Done => &mut todo.done,
        }
    }

    pub fn get_sorting(&self, config: &ToDoConfig) -> TaskSort {
        use ToDoData::*;
        match self {
            Pending => config.pending_sort,
            Done => config.done_sort,
        }
    }
}

/// Enum to represent different categories.
#[derive(Clone, Copy, PartialEq)]
pub enum ToDoCategory {
    Projects,
    Contexts,
    Hashtags,
}

impl ToDoCategory {
    pub fn get_data<'a>(&self, task: &'a Task) -> &'a [String] {
        use ToDoCategory::*;
        match self {
            Projects => task.projects(),
            Contexts => task.contexts(),
            Hashtags => &task.hashtags,
        }
    }

    pub fn get_all() -> &'static [ToDoCategory] {
        use ToDoCategory::*;
        static ALL_CATEGORIES: [ToDoCategory; 3] = [Projects, Contexts, Hashtags];
        &ALL_CATEGORIES
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum FilterState {
    Select,
    Remove,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct ToDoState {
    pub active: Option<(ToDoData, usize)>,
    pub project_filters: BTreeMap<String, FilterState>,
    pub context_filters: BTreeMap<String, FilterState>,
    pub hashtag_filters: BTreeMap<String, FilterState>,
}

impl ToDoState {
    pub fn get_category(&self, category: ToDoCategory) -> &BTreeMap<String, FilterState> {
        use ToDoCategory::*;
        match category {
            Projects => &self.project_filters,
            Contexts => &self.context_filters,
            Hashtags => &self.hashtag_filters,
        }
    }

    pub fn get_mut_category(
        &mut self,
        category: ToDoCategory,
    ) -> &mut BTreeMap<String, FilterState> {
        use ToDoCategory::*;
        match category {
            Projects => &mut self.project_filters,
            Contexts => &mut self.context_filters,
            Hashtags => &mut self.hashtag_filters,
        }
    }

    pub fn filter_out(&self, task: &Task) -> bool {
        fn filter(category: &BTreeMap<String, FilterState>, task_categories: &[String]) -> bool {
            category.iter().all(|(category, state)| {
                let contains = task_categories.contains(category);
                match state {
                    FilterState::Select => contains,
                    FilterState::Remove => !contains,
                }
            })
        }
        filter(&self.project_filters, task.projects())
            && filter(&self.context_filters, task.contexts())
            && filter(&self.hashtag_filters, &task.hashtags)
    }

    pub fn set_filter(&mut self, category: ToDoCategory, filter: &str, filter_state: FilterState) {
        let category = self.get_mut_category(category);
        match category.get_mut(filter) {
            Some(a) => {
                if filter_state == *a {
                    category.remove(filter);
                } else {
                    *a = filter_state;
                }
            }
            None => {
                category.insert(filter.to_owned(), filter_state);
            }
        }
    }
}
