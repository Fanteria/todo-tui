mod colors;
mod keycode;
mod logger;
mod styles;
mod text_modifier;
mod text_style;
mod todo_config;

pub use self::keycode::KeyCodeDef;
pub use self::logger::Logger;
pub use self::styles::Styles;
pub use self::styles::StylesValue;
pub use self::text_style::TextStyle;
pub use self::text_style::TextStyleList;
pub use self::todo_config::ToDoConfig;

use self::colors::opt_color;
use crate::{
    layout::widget::widget_type::WidgetType,
    todo::task_list::TaskSort,
    ui::{EventHandlerUI, UIEvent},
};
use clap::{arg, CommandFactory, Parser};

use clap_complete::{generate, shells::Bash};
use crossterm::event::KeyCode;
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env::var,
    error::Error,
    fs::File,
    io::{self, Read, Write},
    num::ParseIntError,
    path::PathBuf,
    time::Duration,
};
use tui::style::Color;

/// Configuration struct for the ToDo TUI application.
#[derive(Serialize, Deserialize, Default, Parser)]
#[command(author, version, about, long_about = None)]
#[cfg_attr(test, derive(PartialEq, Debug))]
pub struct Config {
    /// Path to configuration file.
    #[serde(skip)]
    #[arg(short, long, value_name = "FILE")]
    config_path: Option<PathBuf>,

    /// Generate autocomplete script to given file path.
    #[serde(skip)]
    #[arg(long, value_name = "FILE", help_heading = "export")]
    generate_autocomplete: Option<PathBuf>,

    /// Generate full configuration file for actual session
    /// so present configuration file and command lines
    /// options are taken in account.
    #[serde(skip)]
    #[arg(long, value_name = "FILE", help_heading = "export")]
    export_config: Option<PathBuf>,

    /// Generate configuration file with default values
    /// to given file path.
    #[serde(skip)]
    #[arg(long, value_name = "FILE", help_heading = "export")]
    export_default_config: Option<PathBuf>,

    #[serde(default, with = "opt_color")]
    #[arg(long, value_name = "COLOR")]
    active_color: Option<Color>,

    /// Widget that will be active after start of the application.
    #[arg(short, long, value_name = "WIDGET_TYPE")]
    init_widget: Option<WidgetType>,

    /// Title of window with opened todo-tui {env!("CARGO_PKG_NAME")} {AAAA}
    #[arg(short = 'T', long, value_name = "STRING")]
    window_title: Option<String>,

    #[arg(short, long, value_name = "STRING")]
    todo_path: Option<String>,

    #[arg(short, long, value_name = "STRING")]
    archive_path: Option<String>,

    #[arg(long)] // TODO value type
    priority_colors: Option<TextStyleList>,

    #[arg(short, long, value_name = "FLAG")]
    wrap_preview: Option<bool>,

    #[arg(long, value_name = "TEXT_STYLE")]
    list_active_color: Option<TextStyle>,

    #[arg(long, value_name = "TEXT_STYLE")]
    pending_active_color: Option<TextStyle>,

    #[arg(long, value_name = "TEXT_STYLE")]
    done_active_color: Option<TextStyle>,

    #[arg(short = 'd', long, value_parser = parse_duration, value_name = "DURATION")]
    autosave_duration: Option<Duration>,

    #[arg(long, value_name = "FILE", help_heading = "export")]
    save_state_path: Option<PathBuf>,

    #[arg(long, value_name = "FILE")]
    log_file: Option<PathBuf>,

    #[arg(long, value_name = "FILE")]
    log_format: Option<String>,

    #[arg(long, value_name = "LOG_LEVEL")]
    log_level: Option<LevelFilter>,

    #[arg(short, long, value_name = "FLAG")]
    file_watcher: Option<bool>,

    #[arg(short = 'L', long, value_parser = parse_duration, value_name = "DURATION")]
    list_refresh_rate: Option<Duration>,

    #[arg(short, long, value_name = "NUMBER")]
    list_shift: Option<usize>,

    #[arg(long, value_name = "TASK_SORT")]
    pending_sort: Option<TaskSort>,

    #[arg(long, value_name = "TASK_SORT")]
    done_sort: Option<TaskSort>,

    #[arg(short, long, value_name = "STRING")]
    preview_format: Option<String>,

    #[arg(long, value_name = "STRING")]
    layout: Option<String>,

    #[clap(skip)]
    tasks_keybind: Option<EventHandlerUI>,

    #[clap(skip)]
    category_keybind: Option<EventHandlerUI>,

    #[clap(skip)]
    list_keybind: Option<EventHandlerUI>,

    #[clap(skip)]
    window_keybind: Option<EventHandlerUI>,

    #[arg(long, value_name = "TEXT_STYLE")]
    category_style: Option<TextStyle>,

    #[arg(long, value_name = "TEXT_STYLE")]
    category_select_style: Option<TextStyle>,

    #[arg(long, value_name = "TEXT_STYLE")]
    category_remove_style: Option<TextStyle>,

    #[arg(long, value_name = "TEXT_STYLE")]
    projects_style: Option<TextStyle>,

    #[arg(long, value_name = "TEXT_STYLE")]
    contexts_style: Option<TextStyle>,

    #[arg(long, value_name = "TEXT_STYLE")]
    hashtags_style: Option<TextStyle>,

    #[clap(skip)]
    custom_category_style: Option<HashMap<String, TextStyle>>,
}

impl Config {

    pub fn new() -> Self {
        let mut config = Config::parse();
        if let Ok(load_config) = config.load_config() {
            config = config.merge(load_config);
        }
        config
    }

    /// Loads the default configuration settings.
    ///
    /// This function first attempts to load the configuration file, and if it fails, it returns the default configuration.
    ///
    /// # Returns
    ///
    /// A `Result` containing the loaded configuration (`Ok`) or an error (`Err`) if loading fails.
    pub fn load(path: &PathBuf) -> io::Result<Self> {
        Ok(Self::load_from_buffer(File::open(path)?))
    }

    pub fn load_config(&self) -> io::Result<Self> {
        match &self.config_path {
            Some(path) => Config::load(path),
            None => Self::load_default(),
        }
    }

    /// Returns the default configuration file path based on environment variables.
    ///
    /// The configuration file path is determined based on the XDG_CONFIG_HOME and HOME environment variables.
    ///
    /// # Returns
    ///
    /// A `Result` containing the default configuration file path (`Ok`) or an error (`Err`) if the path cannot be determined.
    pub fn load_default() -> io::Result<Self> {
        const CONFIG_FOLDER: &str = "/.config/";
        const CONFIG_NAME: &str = "todo-tui.toml";
        let path = var("XDG_CONFIG_HOME")
            .or_else(|_| var("HOME").map(|home| format!("{home}{CONFIG_FOLDER}")))
            .unwrap_or(String::from("~") + CONFIG_FOLDER)
            + CONFIG_NAME;
        Ok(Self::load_from_buffer(File::open(path)?))
    }

    /// Loads a configuration from a provided reader.
    ///
    /// # Parameters
    ///
    /// - `reader`: A reader implementing the `Read` trait.
    ///
    /// # Returns
    ///
    /// The loaded configuration.
    pub fn load_from_buffer<R>(mut reader: R) -> Self
    where
        R: Read,
    {
        let mut buf = String::default();
        if let Err(e) = reader.read_to_string(&mut buf) {
            log::error!("Cannot load config: {}", e);
            return Self::default();
        }
        match toml::from_str(buf.as_str()) {
            Ok(c) => c,
            Err(e) => {
                log::error!("Cannot parse config: {}", e);
                Self::default()
            }
        }
    }

    pub fn merge(self, other: Config) -> Self {
        Self {
            config_path: self.config_path.or(other.config_path),
            generate_autocomplete: self.generate_autocomplete.or(other.generate_autocomplete),
            export_config: self.export_config.or(other.export_config),
            export_default_config: self.export_default_config.or(other.export_default_config),
            active_color: self.active_color.or(other.active_color),
            init_widget: self.init_widget.or(other.init_widget),
            window_title: self.window_title.or(other.window_title),
            todo_path: self.todo_path.or(other.todo_path),
            archive_path: self.archive_path.or(other.archive_path),
            priority_colors: self.priority_colors.or(other.priority_colors),
            wrap_preview: self.wrap_preview.or(other.wrap_preview),
            list_active_color: self.list_active_color.or(other.list_active_color),
            pending_active_color: self.pending_active_color.or(other.pending_active_color),
            done_active_color: self.done_active_color.or(other.done_active_color),
            autosave_duration: self.autosave_duration.or(other.autosave_duration),
            save_state_path: self.save_state_path.or(other.save_state_path),
            log_file: self.log_file.or(other.log_file),
            log_format: self.log_format.or(other.log_format),
            log_level: self.log_level.or(other.log_level),
            file_watcher: self.file_watcher.or(other.file_watcher),
            list_refresh_rate: self.list_refresh_rate.or(other.list_refresh_rate),
            list_shift: self.list_shift.or(other.list_shift),
            pending_sort: self.pending_sort.or(other.pending_sort),
            done_sort: self.done_sort.or(other.done_sort),
            preview_format: self.preview_format.or(other.preview_format),
            layout: self.layout.or(other.layout),
            tasks_keybind: self.tasks_keybind.or(other.tasks_keybind),
            category_keybind: self.category_keybind.or(other.category_keybind),
            list_keybind: self.list_keybind.or(other.list_keybind),
            window_keybind: self.window_keybind.or(other.window_keybind),
            category_style: self.category_style.or(other.category_style),
            category_select_style: self.category_select_style.or(other.category_select_style),
            category_remove_style: self.category_remove_style.or(other.category_remove_style),
            projects_style: self.projects_style.or(other.projects_style),
            contexts_style: self.contexts_style.or(other.contexts_style),
            hashtags_style: self.hashtags_style.or(other.hashtags_style),
            custom_category_style: self.custom_category_style.or(other.custom_category_style),
        }
    }

    pub fn fill(&self) -> Self {
        Self {
            config_path: self.config_path.clone(),
            generate_autocomplete: self.generate_autocomplete.clone(),
            export_config: self.export_config.clone(),
            export_default_config: self.export_default_config.clone(),
            active_color: Some(self.get_active_color()),
            init_widget: Some(self.get_init_widget()),
            window_title: Some(self.get_window_title()),
            todo_path: Some(self.get_todo_path()),
            archive_path: self.get_archive_path(),
            priority_colors: Some(self.get_priority_colors()),
            wrap_preview: Some(self.get_wrap_preview()),
            list_active_color: Some(self.get_list_active_color()),
            pending_active_color: Some(self.get_pending_active_color()),
            done_active_color: Some(self.get_done_active_color()),
            autosave_duration: Some(self.get_autosave_duration()),
            save_state_path: self.get_save_state_path(),
            log_file: Some(self.get_log_file()),
            log_format: Some(self.get_log_format()),
            log_level: Some(self.get_log_level()),
            file_watcher: Some(self.get_file_watcher()),
            list_refresh_rate: Some(self.get_list_refresh_rate()),
            list_shift: Some(self.get_list_shift()),
            pending_sort: Some(self.get_pending_sort()),
            done_sort: Some(self.get_done_sort()),
            preview_format: Some(self.get_preview_format()),
            layout: Some(self.get_layout()),
            tasks_keybind: Some(self.get_tasks_keybind()),
            category_keybind: Some(self.get_category_keybind()),
            list_keybind: Some(self.get_list_keybind()),
            window_keybind: Some(self.get_window_keybind()),
            category_style: Some(self.get_category_style()),
            category_select_style: Some(self.get_category_select_style()),
            category_remove_style: Some(self.get_category_remove_style()),
            projects_style: Some(self.get_projects_style()),
            contexts_style: Some(self.get_contexts_style()),
            hashtags_style: Some(self.get_hashtags_style()),
            custom_category_style: Some(self.get_custom_category_style()),
        }
    }

    pub fn export(&self) -> Result<bool, Box<dyn Error>> {
        let mut ret = false;
        if let Some(path) = &self.generate_autocomplete {
            generate(
                Bash,
                &mut Self::command(),
                env!("CARGO_PKG_NAME"),
                &mut File::create(path)?,
            );
            ret = true
        }
        if let Some(path) = &self.export_config {
            let mut output = File::create(path)?;
            write!(output, "{}", toml::to_string_pretty(&self.fill())?)?;
            ret = true
        }
        if let Some(path) = &self.export_default_config {
            let mut output = File::create(path)?;
            write!(
                output,
                "{}",
                toml::to_string_pretty(&Config::default().fill())?
            )?;
            ret = true
        }
        Ok(ret)
    }

    pub fn get_active_color(&self) -> Color {
        self.active_color.unwrap_or(Color::Red)
    }

    pub fn get_init_widget(&self) -> WidgetType {
        self.init_widget.unwrap_or(WidgetType::List)
    }

    pub fn get_window_title(&self) -> String {
        self.window_title
            .clone()
            .unwrap_or(String::from("ToDo tui"))
    }

    pub fn get_todo_path(&self) -> String {
        self.todo_path
            .clone()
            .unwrap_or(var("HOME").unwrap_or(String::from("~")) + "/todo.txt")
    }

    pub fn get_archive_path(&self) -> Option<String> {
        self.archive_path.clone()
    }

    fn get_priority_colors(&self) -> TextStyleList {
        self.priority_colors.clone().unwrap_or_default()
    }

    pub fn get_wrap_preview(&self) -> bool {
        self.wrap_preview.unwrap_or(true)
    }

    pub fn get_list_active_color(&self) -> TextStyle {
        self.list_active_color
            .unwrap_or(TextStyle::default().bg(Color::LightRed))
    }

    pub fn get_pending_active_color(&self) -> TextStyle {
        self.pending_active_color.unwrap_or_default()
    }

    pub fn get_done_active_color(&self) -> TextStyle {
        self.done_active_color.unwrap_or_default()
    }

    pub fn get_autosave_duration(&self) -> Duration {
        self.autosave_duration.unwrap_or(Duration::from_secs(900))
    }

    pub fn get_save_state_path(&self) -> Option<PathBuf> {
        self.save_state_path.clone()
    }

    fn get_log_file(&self) -> PathBuf {
        self.log_file.clone().unwrap_or(PathBuf::from("log.log"))
    }

    fn get_log_format(&self) -> String {
        self.log_format
            .clone()
            .unwrap_or(String::from("{d} [{h({l})}] {M}: {m}{n}"))
    }

    fn get_log_level(&self) -> LevelFilter {
        self.log_level.unwrap_or(LevelFilter::Info)
    }

    pub fn get_file_watcher(&self) -> bool {
        self.file_watcher.unwrap_or(true)
    }

    pub fn get_list_refresh_rate(&self) -> Duration {
        self.list_refresh_rate.unwrap_or(Duration::from_secs(5))
    }

    pub fn get_list_shift(&self) -> usize {
        self.list_shift.unwrap_or(4)
    }

    pub fn get_pending_sort(&self) -> TaskSort {
        self.pending_sort.unwrap_or(TaskSort::None)
    }

    pub fn get_done_sort(&self) -> TaskSort {
        self.done_sort.unwrap_or(TaskSort::None)
    }

    pub fn get_preview_format(&self) -> String {
        self.preview_format.clone().unwrap_or(String::from(
            "Pending: $pending Done: $done
Subject: $subject
Priority: $priority
Create date: $create_date
Link: $link",
        ))
    }

    pub fn get_layout(&self) -> String {
        self.layout.clone().unwrap_or(String::from(
            "
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
",
        ))
    }

    pub fn get_tasks_keybind(&self) -> EventHandlerUI {
        self.tasks_keybind.clone().unwrap_or(EventHandlerUI::new(&[
            (KeyCode::Char('U'), UIEvent::SwapUpItem),
            (KeyCode::Char('D'), UIEvent::SwapDownItem),
            (KeyCode::Char('x'), UIEvent::RemoveItem),
            (KeyCode::Char('d'), UIEvent::MoveItem),
            (KeyCode::Enter, UIEvent::Select),
        ]))
    }

    pub fn get_category_keybind(&self) -> EventHandlerUI {
        self.category_keybind
            .clone()
            .unwrap_or(EventHandlerUI::new(&[
                (KeyCode::Enter, UIEvent::Select),
                (KeyCode::Backspace, UIEvent::Remove),
            ]))
    }

    pub fn get_list_keybind(&self) -> EventHandlerUI {
        self.list_keybind.clone().unwrap_or(EventHandlerUI::new(&[
            (KeyCode::Char('j'), UIEvent::ListDown),
            (KeyCode::Char('k'), UIEvent::ListUp),
            (KeyCode::Char('g'), UIEvent::ListFirst),
            (KeyCode::Char('G'), UIEvent::ListLast),
        ]))
    }

    pub fn get_window_keybind(&self) -> EventHandlerUI {
        self.window_keybind.clone().unwrap_or(EventHandlerUI::new(&[
            (KeyCode::Char('q'), UIEvent::Quit),
            (KeyCode::Char('S'), UIEvent::Save),
            (KeyCode::Char('u'), UIEvent::Load),
            (KeyCode::Char('H'), UIEvent::MoveLeft),
            (KeyCode::Char('L'), UIEvent::MoveRight),
            (KeyCode::Char('K'), UIEvent::MoveUp),
            (KeyCode::Char('J'), UIEvent::MoveDown),
            (KeyCode::Char('I'), UIEvent::InsertMode),
            (KeyCode::Char('E'), UIEvent::EditMode),
        ]))
    }

    fn get_category_style(&self) -> TextStyle {
        self.category_style.unwrap_or_default()
    }

    fn get_category_select_style(&self) -> TextStyle {
        self.category_select_style.unwrap_or_else(|| {
            TextStyle::default().fg(Color::Green)
        })
    }

    fn get_category_remove_style(&self) -> TextStyle {
        self.category_remove_style.unwrap_or_else(|| {
            TextStyle::default().fg(Color::Red)
        })
    }

    fn get_projects_style(&self) -> TextStyle {
        self.projects_style.unwrap_or_default()
    }

    fn get_contexts_style(&self) -> TextStyle {
        self.contexts_style.unwrap_or_default()
    }

    fn get_hashtags_style(&self) -> TextStyle {
        self.hashtags_style.unwrap_or_default()
    }

    fn get_custom_category_style(&self) -> HashMap<String, TextStyle> {
        let default = || {
            let mut custom_category_style = HashMap::new();
            custom_category_style.insert(
                String::from("+todo-tui"),
                TextStyle::default().fg(Color::LightBlue),
            );
            custom_category_style
        };
        self.custom_category_style.clone().unwrap_or_else(default)
    }
}

fn parse_duration(arg: &str) -> Result<Duration, ParseIntError> {
    Ok(Duration::from_secs(arg.parse()?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Result;

    #[test]
    fn test_deserialization() {
        let deserialized: Config = toml::from_str(
            r#"
            active_color = "Green"
            init_widget = "Done"
        "#,
        )
        .unwrap();

        assert_eq!(deserialized.active_color, Some(Color::Green));
        assert_eq!(deserialized.init_widget, Some(WidgetType::Done));
        assert_eq!(deserialized.window_title, None);
        assert_eq!(deserialized.get_window_title(), "ToDo tui");
    }

    #[test]
    fn test_serialization() {
        let c = Config::default();
        let serialized = toml::to_string_pretty(&c).unwrap();
        println!("{}", serialized);
        let deserialized: Config = toml::from_str(&serialized).unwrap();
        assert_eq!(c, deserialized);
    }

    #[test]
    fn test_load() -> Result<()> {
        let s = r#"
        active_color = "Blue"
        window_title = "Title"
        todo_path = "path to todo file"
        "#;

        let c = Config::load_from_buffer(s.as_bytes());
        assert_eq!(c.active_color, Some(Color::Blue));
        assert_eq!(c.init_widget, None);
        assert_eq!(c.get_init_widget(), WidgetType::List);
        assert_eq!(c.window_title, Some(String::from("Title")));
        assert_eq!(c.todo_path, Some(String::from("path to todo file")));
        assert_eq!(c.archive_path, None);

        Ok(())
    }

    #[test]
    fn help_can_be_generated() {
        Config::parse();
    }

    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("1000"), Ok(Duration::from_secs(1000)));
        assert!(parse_duration("-1000").is_err());
    }

    #[test]
    fn test_merge() {
        let mut conf1 = Config::default();
        let mut conf2 = Config::default();
        conf1.todo_path = Some("path/to/todo/file".to_string());
        conf2.archive_path = Some("path/to/archive_path/file".to_string());

        conf1.window_title = Some("Window title".to_string());
        conf2.window_title = Some("Different title".to_string());

        let new_conf = conf1.merge(conf2);
        assert_eq!(new_conf.todo_path, Some("path/to/todo/file".to_string()));
        assert_eq!(new_conf.archive_path, Some("path/to/archive_path/file".to_string()));
        assert_eq!(new_conf.window_title, Some("Window title".to_string()));
    }
}
