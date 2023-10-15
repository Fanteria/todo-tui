use std::str::FromStr;

use super::{widget_base::WidgetBase, widget_trait::State};
use crate::{todo::ToDoData, ui::UIEvent, CONFIG};
use chrono::NaiveDate;
use tui::{
    backend::Backend,
    style::{Style, Color, Modifier},
    widgets::{Paragraph, Wrap},
    Frame,
};

#[derive(Debug, PartialEq, Eq)]
enum Parts {
    Text(String),
    Pending,
    Done,
    Subject,
    Priority,
    CreateDate,
    FinishDate,
    Finished,
    TresholdDate,
    DueDate,
    Contexts,
    Projects,
    Hashtags,
    Special(String),
}

impl From<String> for Parts {
    fn from(value: String) -> Self {
        use Parts::*;
        match value.to_lowercase().as_str() {
            "pending" => Pending,
            "done" => Done,
            "subject" => Subject,
            "priority" => Priority,
            "createDate" => CreateDate,
            "finishDate" => FinishDate,
            "finished" => Finished,
            "treshold_date" => TresholdDate,
            "dueDate" => DueDate,
            "contexts" => Contexts,
            "projects" => Projects,
            "hashtags" => Hashtags,
            _ => Special(value),
        }
    }
}

/// Represents the state for a preview widget that displays task details.
pub struct StatePreview {
    base: WidgetBase,
    format: String,
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
    pub fn new(base: WidgetBase, format: String) -> Self {
        StatePreview { format, base }
    }

    /// Generates the content for the preview based on the current task data.
    ///
    /// # Returns
    ///
    /// A string containing the formatted task details.
    fn get_content(&self) -> String {
        let data = self.base.data();
        let task = match data.get_active() {
            Some(s) => s,
            None => return String::from(""),
        };
        let date_to_str = |date: Option<NaiveDate>| match date {
            Some(date) => date.to_string(),
            None => String::from(""),
        };
        // TODO remove
        let mut content = self.format.clone();
        let parsed = StatePreview::parse(&self.format);
        println!("{:?}", parsed);
        // -----
        task.tags.iter().for_each(|(key, value)| {
            content = content.replace(&("{".to_string() + key + "}"), value);
        });
        content
            .replace("{n}", &data.len(ToDoData::Pending).to_string())
            .replace("{N}", &data.len(ToDoData::Done).to_string())
            .replace("{s}", &task.subject)
            .replace("{p}", &task.priority.to_string())
            .replace("{c}", &date_to_str(task.create_date))
            .replace("{f}", &date_to_str(task.finish_date))
            .replace("{F}", &task.finished.to_string())
            .replace("{t}", &date_to_str(task.threshold_date))
            .replace("{d}", &date_to_str(task.due_date))
            .replace("{C}", &task.contexts().join(", "))
            .replace("{P}", &task.projects().join(", "))
            .replace("{H}", &task.hashtags.join(", "))
    }

    fn read_block(iter: &mut std::str::Chars<'_>, delimiter: char) -> String {
        let mut read = String::default();
        loop {
            let c = match iter.next() {
                Some(c) => c,
                None => break, // TODO errror, block is not closed
            };
            match c {
                '\\' => read.push(iter.next().unwrap()),
                c if c == delimiter => break,
                _ => read.push(c),
            };
        }
        read
    }

    fn parse_variables(block: &str) -> Vec<Parts> {
        let mut ret = Vec::new();
        let mut iter = block.chars();
        let mut read_variable = false;
        let mut variable_block = false;
        let mut read = String::new();
        loop {
            let c = match iter.next() {
                Some(c) => c,
                None => break,
            };
            match c {
                '$' => {
                    read_variable = true;
                    ret.push(Parts::Text(read));
                    read = String::new();
                    match iter.next() {
                        Some('{') => variable_block = true,
                        Some(ch) => read.push(ch),
                        None => {} // TODO error, empty variable name
                    }
                }
                '}' if read_variable && variable_block => {
                    variable_block = false;
                    read_variable = false;
                    ret.push(Parts::from(read));
                    read = String::new();
                }
                c if read_variable && !variable_block && c.is_whitespace() => {
                    read_variable = false;
                    ret.push(Parts::from(read));
                    read = String::new();
                }
                '\\' => read.push(iter.next().unwrap()),
                _ => read.push(c),
            };
        }
        ret.push(if read_variable {
            if variable_block {
                todo!() // TODO error variable block not closed
            }
            Parts::from(read)
        } else {
            Parts::Text(read)
        });

        ret
    }

    fn parse_style(style: Option<String>) -> Style {
        let mut ret = Style::default();
        if let Some(style) = style {
            style.split_whitespace().for_each(|word| {
                let color = Color::from_str(word);
                let modifier = Modifier::from_name(word);

            });
        }
        // Style::default().mo
        
        ret
    }

    fn parse(template: &str) -> Vec<(Vec<Parts>, Style)> {
        let mut ret = Vec::new();
        let mut act = String::default();
        let mut iter = template.chars().into_iter();
        loop {
            let c = match iter.next() {
                Some(c) => c,
                None => break,
            };
            match c {
                '[' => {
                    let block = &StatePreview::read_block(&mut iter, ']');
                    let mut style = None;
                    match iter.next() {
                        Some('(') => {
                            style = Some(StatePreview::read_block(&mut iter, ')'));
                        }
                        Some('\\') => act.push(iter.next().unwrap()),
                        Some(ch) => act.push(ch),
                        None => break,
                    }
                    ret.push((
                        StatePreview::parse_variables(&block),
                        StatePreview::parse_style(style),
                    ));
                }
                '\\' => act.push(iter.next().unwrap()),
                _ => act.push(c),
            }
        }
        // ahead = "[$count](bold white) "

        ret
    }
}

impl State for StatePreview {
    fn handle_event_state(&mut self, _: UIEvent) -> bool {
        false
    }

    fn render<B: Backend>(&self, f: &mut Frame<B>) {
        let mut paragraph = Paragraph::new(self.get_content()).block(self.get_block());
        if CONFIG.wrap_preview {
            paragraph = paragraph.wrap(Wrap { trim: true })
        }
        // .style(Style::default().fg(Color::White).bg(Color::Black));
        // .alignment(Alignment::Center)
        f.render_widget(paragraph, self.base.chunk);
    }

    fn get_base(&self) -> &WidgetBase {
        &self.base
    }

    fn get_base_mut(&mut self) -> &mut WidgetBase {
        &mut self.base
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_block() {
        let mut iter = "block to parse]".chars();
        assert_eq!(&StatePreview::read_block(&mut iter, ']'), "block to parse");
        assert_eq!(&iter.collect::<String>(), "");

        let mut iter = "Some style block)".chars();
        assert_eq!(
            &StatePreview::read_block(&mut iter, ')'),
            "Some style block"
        );
        assert_eq!(&iter.collect::<String>(), "");

        let mut iter = "block to parse] some other text".chars();
        assert_eq!(&StatePreview::read_block(&mut iter, ']'), "block to parse");
        assert_eq!(&iter.collect::<String>(), " some other text");
    }

    #[test]
    fn parse_variables() {
        let parts = StatePreview::parse_variables("");
        assert_eq!(parts[0], Parts::Text("".into()));

        let parts = StatePreview::parse_variables("some text");
        assert_eq!(parts[0], Parts::Text("some text".into()));

        let parts = StatePreview::parse_variables("some text $done another text");
        assert_eq!(parts[0], Parts::Text("some text ".into()));
        assert_eq!(parts[1], Parts::Done);
        assert_eq!(parts[2], Parts::Text("another text".into()));

        let parts = StatePreview::parse_variables("there is ${pending}x pending tasks");
        assert_eq!(parts[0], Parts::Text("there is ".into()));
        assert_eq!(parts[1], Parts::Pending);
        assert_eq!(parts[2], Parts::Text("x pending tasks".into()));

        let parts = StatePreview::parse_variables("spacial task text $some-special");
        assert_eq!(parts[0], Parts::Text("spacial task text ".into()));
        assert_eq!(parts[1], Parts::Special("some-special".into()));
    }

    #[test]
    fn parse_stype() {

    }
}
