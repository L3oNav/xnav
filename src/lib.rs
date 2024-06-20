// src/lib.rs
// Uncomment if you plan to use these features
// #![feature(ptr_from_ref)]
// #![feature(is_some_and)]

pub mod config;
pub mod server;
pub mod service;
pub mod sync;
pub mod threading;

use std::io;

pub use config::{Action, Algorithm, Backend, Config, Forward, Pattern, Server};
pub use server::{Master, Server as ServerInstance, ShutdownState, State};
pub use service::{BoxBodyResponse, LocalResponse, ProxyResponse};
pub use sync::{Notification, Notifier, Subscription};
pub use threading::{make as make_scheduler, Scheduler, WeightedRoundRobin};

/// RXH version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Top level error to use for return types in the public API and main function.
#[derive(Debug)]
pub enum Error {
    /// Mostly related to reading or writing on sockets.
    Io(io::Error),

    /// An error while deserializing the config file.
    Toml(toml::de::Error),

    /// Error while processing HTTP requests.
    Http(hyper::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(err) => write!(f, "IO error: {err}"),
            Error::Toml(err) => write!(f, "TOML parse error: {err}"),
            Error::Http(err) => write!(f, "HTTP error: {err}"),
        }
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Error::Io(value)
    }
}

impl From<toml::de::Error> for Error {
    fn from(value: toml::de::Error) -> Self {
        Error::Toml(value)
    }
}

impl From<hyper::Error> for Error {
    fn from(value: hyper::Error) -> Self {
        Error::Http(value)
    }
}
