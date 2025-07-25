pub mod core;
pub mod cli;
pub mod web;

pub use core::DiffyCore;
pub use cli::TuiApp;
pub use web::{create_app, start_server};