//! Load balancing and scheduler implementations.
mod wrr;

pub use wrr::WeightedRoundRobin;

use crate::config::{Algorithm, Backend};

/// A scheduler provides an algorithm for load balancing between multiple
/// backend servers.
pub trait Scheduler {
    /// Returns the address of the server that should process the next request.
    fn next_server(&self) -> std::net::SocketAddr;
}

/// [`Scheduler`] factory.
pub fn make(algorithm: Algorithm, backends: &Vec<Backend>) -> Box<dyn Scheduler + Send + Sync> {
    Box::new(match algorithm {
        Algorithm::Wrr => WeightedRoundRobin::new(backends),
    })
}
