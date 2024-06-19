//! This module defines the main server architecture, organizing tasks and handling requests.

mod main;
mod server;

pub use main::Master;
pub use server::{Server, ShutdownState, State};
