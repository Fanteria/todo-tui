mod config;
mod error;
mod file_worker;
mod layout;
mod todo;
mod ui;

use crate::{
    config::{Config, Logger},
    todo::ToDo,
    ui::UI,
};
use std::error::Error;

#[macro_use]
extern crate enum_dispatch;

fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::new();
    if !config.export()? {
        Logger::new(&config).init()?;
        log::trace!("===== START LOGGING =====");
        let mut ui = UI::build(&config)?;
        log::trace!("===== STARING UI =====");
        ui.run()?;
    }
    Ok(())
}

