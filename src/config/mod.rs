//! Structs and enums derived from the config file using [`serde`].
mod config;
pub use config::{Action, Algorithm, Backend, Config, Forward, Pattern, Server};
