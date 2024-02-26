mod ui_event;
mod ui_state;

pub use ui_event::*;
pub use ui_state::*;

use crate::{
    config::Config,
    file_worker::{FileWorker, FileWorkerCommands},
    layout::Layout,
    layout::Render,
    todo::autocomplete,
    ToDo,
};
use crossterm::{
    self,
    event::{self, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
    },
    ExecutableCommand,
};
use std::{
    error::Error,
    io::{self, Result as ioResult},
    sync::mpsc::Sender,
    sync::{Arc, Mutex},
    time::Duration,
    path::PathBuf,
};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout as tuiLayout, Rect},
    style::{Color, Style},
    widgets::Paragraph,
    widgets::{Block, BorderType, Borders},
    Terminal,
};
use tui_input::{backend::crossterm::EventHandler, Input};

/// Enum representing the different modes of the UI.
#[derive(Debug, PartialEq, Eq)]
enum Mode {
    Input,
    Edit,
    Normal,
}

/// The struct representing the UI for the application.
pub struct UI {
    input_chunk: Rect,
    tinput: Input,
    layout: Layout,
    mode: Mode,
    data: Arc<Mutex<ToDo>>,
    tx: Sender<FileWorkerCommands>,
    event_handler: EventHandlerUI,
    quit: bool,
    window_title: String,
    list_refresh_rate: Duration,
    active_color: Color,
    save_state_path: Option<PathBuf>,
}

impl UI {
    /// Creates a new instance of the UI.
    ///
    /// # Arguments
    ///
    /// * `layout` - The initial layout configuration for the UI.
    /// * `data` - Shared data representing the to-do list.
    /// * `tx` - Sender for communicating with the file worker.
    ///
    /// # Returns
    ///
    /// A new `UI` instance.
    pub fn new(
        layout: Layout,
        data: Arc<Mutex<ToDo>>,
        tx: Sender<FileWorkerCommands>,
        config: &Config,
    ) -> UI {
        UI {
            input_chunk: Rect::default(),
            tinput: Input::default(),
            layout,
            mode: Mode::Normal,
            data,
            tx,
            event_handler: config.get_window_keybind(),
            quit: false,
            window_title: config.get_window_title(),
            list_refresh_rate: config.get_list_refresh_rate(),
            active_color: config.get_active_color(),
            save_state_path: config.get_save_state_path(),
        }
    }

    pub fn build(config: &Config) -> Result<UI, Box<dyn Error>> {
        let mut todo = ToDo::new(&config);

        if let Some(path) = &config.get_save_state_path() {
            let state = UIState::load(path);
            let (_active, todo_state) = (state.active, state.todo_state);
            todo.update_state(todo_state);
        }

        let todo = Arc::new(Mutex::new(todo));
        let file_worker = FileWorker::new(
            config.get_todo_path(),
            config.get_archive_path(),
            todo.clone(),
        );

        file_worker.load()?;
        let tx = file_worker.run(config.get_autosave_duration(), config.get_file_watcher());

        let layout = Layout::from_str(&config.get_layout(), todo.clone(), &config)?;


        Ok(UI::new(
            layout,
            todo,
            tx.clone(),
            &config,
        ))
    }

    /// Updates the input chunk of the UI based on the main chunk's dimensions.
    ///
    /// This method recalculates the position and size of the input chunk based on the dimensions
    /// of the main chunk, ensuring proper rendering of the input field.
    ///
    /// # Arguments
    ///
    /// * `main_chunk` - The main chunk's dimensions, typically representing the entire terminal window.
    fn update_chunk(&mut self, main_chunk: Rect) {
        let layout = tuiLayout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(main_chunk);
        self.input_chunk = layout[0];
        self.layout.update_chunk(layout[1]);
    }

    /// Runs the user interface, handling setup and cleanup of terminal interactions.
    ///
    /// This method enables raw mode, sets up the terminal, and enters the main event loop.
    ///
    /// # Returns
    ///
    /// An `ioResult` indicating the success of running the user interface.
    pub fn run(&mut self) -> ioResult<()> {
        fn run_ui(this: &mut UI) -> ioResult<()> {
            // setup terminal
            enable_raw_mode()?;
            let mut stdout = io::stdout();
            execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

            let mut backend = CrosstermBackend::new(stdout);
            backend.execute(SetTitle(this.window_title.clone()))?;

            let mut terminal = Terminal::new(backend)?;
            terminal.hide_cursor()?;
            this.update_chunk(terminal.size()?);

            this.draw(&mut terminal)?;
            this.main_loop(&mut terminal)?;

            // restore terminal
            disable_raw_mode()?;
            execute!(
                terminal.backend_mut(),
                LeaveAlternateScreen,
                DisableMouseCapture
            )?;
            terminal.show_cursor()?;

            Ok(())
        }

        if let Err(e) = run_ui(self) {
            self.tx.send(FileWorkerCommands::Exit).unwrap();
            Err(e)
        } else {
            Ok(())
        }
    }

    /// Handles the main event loop of the UI.
    ///
    /// # Arguments
    ///
    /// * `terminal` - The TUI Terminal.
    ///
    /// # Returns
    ///
    /// An `ioResult` indicating the success of the main loop.
    fn main_loop<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> ioResult<()> {
        let mut version = self.data.lock().unwrap().get_version();
        let mut new_version;
        loop {
            if event::poll(self.list_refresh_rate)? {
                if self.process_event()? {
                    break;
                }
                version = self.data.lock().unwrap().get_version();
                self.draw(terminal)?;
            } else {
                new_version = self.data.lock().unwrap().get_version();
                if new_version != version {
                    version = self.data.lock().unwrap().get_version();
                    self.draw(terminal)?;
                }
            }
        }
        Ok(())
    }

    /// Draws the UI on the terminal.
    ///
    /// # Arguments
    ///
    /// * `terminal` - The TUI Terminal.
    ///
    /// # Returns
    ///
    /// An `ioResult` indicating the success of drawing.
    fn draw<B: Backend>(&self, terminal: &mut Terminal<B>) -> ioResult<()> {
        let mut block = Block::default()
            .borders(Borders::ALL)
            .title("Input")
            .border_type(BorderType::Rounded);
        if self.mode == Mode::Input || self.mode == Mode::Edit {
            block = block.border_style(Style::default().fg(self.active_color));
        }
        terminal.draw(|f| {
            f.render_widget(
                Paragraph::new(self.tinput.value()).block(block),
                self.input_chunk,
            );
            self.layout.render(f);

            if self.mode == Mode::Input || self.mode == Mode::Edit {
                let width = self.input_chunk.width.max(3) - 3;
                let scroll = self.tinput.visual_scroll(width as usize);
                f.set_cursor(
                    self.input_chunk.x
                        + (self.tinput.visual_cursor().max(scroll) - scroll) as u16
                        + 1,
                    self.input_chunk.y + 1,
                );
            }
        })?;
        Ok(())
    }

    /// Handles various user events.
    ///
    /// # Returns
    ///
    /// An `ioResult` indicating whether the application should exit.
    fn process_event(&mut self) -> ioResult<bool> {
        self.handle_event_window(read()?);
        Ok(self.quit)
    }

    fn handle_event_window(&mut self, e: Event) {
        match e {
            Event::Resize(width, height) => {
                log::debug!("Resize event: width {width}, height {height}");
                self.update_chunk(Rect::new(0, 0, width, height));
            }
            Event::Mouse(event) => {
                log::debug!("Mouse event: {:?}", event);
            }
            Event::Key(event) => match self.mode {
                Mode::Input => match event.code {
                    KeyCode::Enter => {
                        self.data
                            .lock()
                            .unwrap()
                            .new_task(self.tinput.value())
                            .unwrap(); // TODO fix
                        self.tinput.reset();
                        self.mode = Mode::Normal;
                        self.layout.focus();
                    }
                    KeyCode::Esc => {
                        self.mode = Mode::Normal;
                        self.layout.focus();
                    }
                    KeyCode::Tab => {
                        if let Some(input) =
                            autocomplete(&self.data.lock().unwrap(), self.tinput.value())
                        {
                            self.tinput = input.into();
                        }
                    }
                    _ => {
                        self.tinput.handle_event(&e);
                    }
                },
                Mode::Edit => match event.code {
                    KeyCode::Enter => {
                        self.data
                            .lock()
                            .unwrap()
                            .update_active(self.tinput.value())
                            .unwrap();
                        self.tinput.reset();
                        self.mode = Mode::Normal;
                        self.layout.focus();
                    }
                    KeyCode::Esc => {
                        self.tinput.reset();
                        self.mode = Mode::Normal;
                        self.layout.focus();
                    }
                    KeyCode::Tab => {
                        if let Some(input) =
                            autocomplete(&self.data.lock().unwrap(), self.tinput.value())
                        {
                            self.tinput = input.into();
                        }
                    }
                    _ => {
                        self.tinput.handle_event(&e);
                    }
                },
                Mode::Normal => {
                    let _ = self.handle_key(&event.code) || self.layout.handle_key(&event);
                }
            },
            _ => {}
        }
    }
}

impl HandleEvent for UI {
    fn get_event(&self, key: &KeyCode) -> UIEvent {
        self.event_handler.get_event(key)
    }

    fn handle_event(&mut self, event: UIEvent) -> bool {
        use UIEvent::*;
        match event {
            Quit => {
                if let Some(path) = &self.save_state_path {
                    if let Err (e) = UIState::new(&self.layout, &self.data.lock().unwrap()).save(path) {
                        log::error!("Error while saveing UI state: {}", e);
                    }
                }
                self.quit = true;
            }
            InsertMode => {
                self.mode = Mode::Input;
                self.layout.unfocus();
            }
            MoveRight => self.layout.right(),
            MoveLeft => self.layout.left(),
            MoveUp => self.layout.up(),
            MoveDown => self.layout.down(),
            Save => {
                if let Err(e) = self.tx.send(FileWorkerCommands::ForceSave) {
                    log::error!("Error while send signal to save todo list: {}", e);
                    // TODO show something on screen
                }
            }
            Load => {
                if let Err(e) = self.tx.send(FileWorkerCommands::Load) {
                    log::error!("Error while send signal to load todo list: {}", e);
                    // TODO show something on screen
                }
            }
            EditMode => {
                if let Some(active) = self.data.lock().unwrap().get_active() {
                    self.tinput = active.to_string().into();
                    self.mode = Mode::Edit;
                    self.layout.unfocus();
                    // self.in
                }
            }
            _ => {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use std::env;
    use crossterm::event::{KeyEvent, KeyModifiers};
    use test_log::test;

    use super::*;

    fn default_ui() -> Result<UI, Box<dyn Error>> {
        let config = Config::load_from_buffer(
            format!(
                r#"
            todo_path = "{}todo.txt"

            [[list_keybind.events]]
            event = "ListDown"
            key.Char = "j"

            [[list_keybind.events]]
            event = "Select"
            key = "Enter"

            [[list_keybind.events]]
            event = "InsertMode"
            key.Char = "I"

            [[list_keybind.events]]
            event = "EditMode"
            key.Char = "E"

            [[list_keybind.events]]
            event = "Quit"
            key.Char = "q"

            [[list_keybind.events]]
            event = "Save"
            key.Char = "S"

            [[list_keybind.events]]
            event = "Load"
            key.Char = "L"
            "#,
                env::var("TODO_TUI_TEST_DIR")?
            )
            .as_bytes(),
        );
        UI::build(&config)
    }

    #[test]
    fn test_behaviour() -> Result<(), Box<dyn Error>> {
        let mut ui = default_ui()?;
        ui.update_chunk(Rect::new(0, 0, 20, 20));

        let event = Event::Resize(50, 50);
        ui.handle_event_window(event);

        let event = Event::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
        ui.handle_event_window(event);

        let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        ui.handle_event_window(event);
        assert!(ui.data.lock().unwrap().get_active().is_some());

        let event = Event::Key(KeyEvent::new(KeyCode::Char('I'), KeyModifiers::NONE));
        ui.handle_event_window(event);
        assert_eq!(ui.mode, Mode::Input);

        let event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        ui.handle_event_window(event);
        assert_eq!(ui.mode, Mode::Normal);

        let event = Event::Key(KeyEvent::new(KeyCode::Char('E'), KeyModifiers::NONE));
        ui.handle_event_window(event);
        assert_eq!(ui.mode, Mode::Edit);

        let event = Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        ui.handle_event_window(event);
        assert_eq!(ui.mode, Mode::Normal);

        assert!(!ui.quit);
        let event = Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE));
        ui.handle_event_window(event);
        assert!(ui.quit);
        ui.quit = false;

        let event = Event::Key(KeyEvent::new(KeyCode::Char('S'), KeyModifiers::NONE));
        ui.handle_event_window(event);

        let event = Event::Key(KeyEvent::new(KeyCode::Char('L'), KeyModifiers::NONE));
        ui.handle_event_window(event);

        Ok(())
    }
}
