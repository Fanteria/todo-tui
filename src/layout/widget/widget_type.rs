use crate::{
    error::ToDoError,
    todo::{ToDoCategory, ToDoData},
};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// An enumeration representing different types of widgets used in the application.
/// Widgets are UI components with specific functionalities, such as task lists, project lists, and previews.
#[derive(Default, PartialEq, Debug, Copy, Clone, Serialize, Deserialize, ValueEnum)]
pub enum WidgetType {
    #[default]
    List,
    Done,
    Project,
    Context,
    Hashtag,
    Preview,
}

impl ToString for WidgetType {
    /// Converts a `WidgetType` variant into its string representation.
    ///
    /// # Returns
    ///
    /// A `String` representing the string name of the `WidgetType` variant.
    fn to_string(&self) -> String {
        use WidgetType::*;
        match self {
            List => String::from("List"),
            Done => String::from("Done"),
            Project => String::from("Projects"),
            Context => String::from("Contexts"),
            Hashtag => String::from("Hashtags"),
            Preview => String::from("Preview"),
        }
    }
}

impl From<ToDoCategory> for WidgetType {
    fn from(value: ToDoCategory) -> Self {
        use ToDoCategory::*;
        match value {
            Projects => WidgetType::Project,
            Contexts => WidgetType::Context,
            Hashtags => WidgetType::Hashtag,
        }
    }
}

impl From<ToDoData> for WidgetType {
    fn from(value: ToDoData) -> Self {
        use ToDoData::*;
        match value {
            Pending => WidgetType::List,
            Done => WidgetType::Done,
        }
    }
}

impl FromStr for WidgetType {
    type Err = ToDoError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use WidgetType::*;
        match s.to_lowercase().as_str() {
            "list" => Ok(List),
            "done" => Ok(Done),
            "projects" => Ok(Project),
            "contexts" => Ok(Context),
            "hashtags" => Ok(Hashtag),
            "preview" => Ok(Preview),
            _ => Err(ToDoError::ParseWidgetType),
        }
    }
}
