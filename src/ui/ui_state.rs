use std::io::{Write, Read, Result as ioResult};
use std::fs::File;
use std::path::Path;

use serde::{Serialize, Deserialize};

use crate::layout::Layout;
use crate::layout::widget::widget_type::WidgetType;
use crate::todo::{ToDo, ToDoState};

#[derive(Default, Serialize, Deserialize)]
pub struct UIState {
    pub active: WidgetType,
    pub todo_state: ToDoState,
}

impl UIState {
    pub fn new(layout: &Layout, todo: &ToDo) -> Self {
        Self {
            active: layout.get_active_widget(),
            todo_state: todo.get_state().clone(),
        }
    }

    pub fn save(&self, path: &Path) -> ioResult<()> {
        self.serialize(&mut File::create(path)?)
    }

    fn serialize<W: Write>(&self, writer: &mut W) -> ioResult<()> {
        writer.write_all(toml::to_string_pretty(&self).unwrap().as_bytes())
    }

    pub fn load(path: &Path) -> Self {
        let file = File::open(path).unwrap(); // TODO remove unwrap here
        UIState::deserialize(file)
    }

    fn deserialize<R: Read>(mut reader: R) -> Self {
        let mut buf = String::default();
        if let Err(e) = reader.read_to_string(&mut buf) {
            log::error!("Cannot load UI state: {}", e);
            return Self::default();
        }
        match toml::from_str(buf.as_str()) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Cannot parse UI state: {}", e);
                Self::default()
            }
        }
    }
}
