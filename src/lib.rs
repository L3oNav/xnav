// src/lib.rs

pub mod server;
pub mod threading;

// Re-export server module to make it easier to access in tests
pub use server::Server;
