use crate::CONFIG;
use serde::{Serialize, Deserialize};
use tui::{
    backend::Backend,
    layout::Rect,
    style::Style,
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

#[allow(dead_code)]
#[derive(PartialEq, Debug, Copy, Clone, Serialize, Deserialize)]
pub enum WidgetType {
    Input,
    List,
    Done,
    Project,
    Context,
}

pub struct Widget {
    pub widget_type: WidgetType,
    pub chunk: Rect,
    pub title: String,
}

impl Widget {
    pub fn new(
        widget_type: WidgetType,
        title: &str,
    ) -> Widget {
        Widget {
            widget_type,
            chunk: Rect{width: 0, height: 0, x: 0, y: 0},
            title: title.to_string(),
        }
    }

    pub fn update_chunk(&mut self, chunk: Rect) {
        self.chunk = chunk;
    }

    pub fn draw<B>(&self, f: &mut Frame<B>, active: bool)
    where
        B: Backend,
    {
        let get_block = || {
            let mut block = Block::default()
                .borders(Borders::ALL)
                .title(self.title.clone())
                .border_type(BorderType::Rounded);
            if active {
                block = block.border_style(Style::default().fg(CONFIG.active_color));
            }
            block
        };

        match self.widget_type {
            WidgetType::Input => {
                f.render_widget(
                    Paragraph::new("Some text").block(get_block()),
                    self.chunk,
                );
            }
            WidgetType::List => {
                f.render_widget(get_block(), self.chunk);
            }
            WidgetType::Done => {
                f.render_widget(get_block(), self.chunk);
            }
            WidgetType::Project => {
                f.render_widget(get_block(), self.chunk);
            }
            WidgetType::Context => {
                f.render_widget(get_block(), self.chunk);
            }
        }
    }
}
