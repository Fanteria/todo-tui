use super::{widget_base::WidgetBase, widget_trait::State};
use crate::{error::ToDoRes, todo::Parser, ui::UIEvent, config::Config};
use std::str::FromStr;
use tui::{
    backend::Backend,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
    Frame,
};

/// Represents the state for a preview widget that displays task details.
pub struct StatePreview {
    base: WidgetBase,
    parser: Parser,
    wrap_preview: bool,
}

impl StatePreview {
    /// Creates a new `StatePreview` instance.
    ///
    /// # Parameters
    ///
    /// - `base`: The base properties shared among different widget types.
    /// - `format`: The format string used to generate the content for the preview.
    ///
    /// # Returns
    ///
    /// A new `StatePreview` instance.
    pub fn new(base: WidgetBase, config: &Config) -> ToDoRes<Self> {
        Ok(StatePreview {
            base,
            parser: Parser::from_str(&config.get_preview_format())?,
            wrap_preview: config.get_wrap_preview(),
        })
    }
}

impl State for StatePreview {
    fn handle_event_state(&mut self, _: UIEvent) -> bool {
        false
    }

    fn render<B: Backend>(&self, f: &mut Frame<B>) {
        let lines = self.parser.fill(&self.base.data());
        let mut paragraph = Paragraph::new(
            lines
                .iter()
                .map(|line| Line {
                    spans: line
                        .iter()
                        .map(|(text, style)| Span::styled(text, *style))
                        .collect::<Vec<_>>(),
                    ..Default::default()
                })
                .collect::<Vec<_>>(),
        )
        .block(self.get_block());
        if self.wrap_preview {
            paragraph = paragraph.wrap(Wrap { trim: true })
        }
        f.render_widget(paragraph, self.base.chunk);
    }

    fn get_base(&self) -> &WidgetBase {
        &self.base
    }

    fn get_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }
}
