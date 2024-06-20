use std::net::SocketAddr;

use super::Scheduler;
use crate::{config::Backend, sync::Ring};

/// Classical Weighted Round Robin (WRR) algorithm.
#[derive(Debug)]
pub struct WeightedRoundRobin {
    /// Pre-computed complete cycle of requests.
    cycle: Ring<SocketAddr>,
}

impl WeightedRoundRobin {
    /// Creates and initializes a new [`WeightedRoundRobin`] scheduler.
    pub fn new(backends: &Vec<Backend>) -> Self {
        let mut cycle = Vec::new();

        // TODO: Interleaved WRR
        for backend in backends {
            let mut weight = backend.weight;
            while weight > 0 {
                cycle.push(backend.address);
                weight -= 1;
            }
        }

        Self {
            cycle: Ring::new(cycle),
        }
    }
}

impl Scheduler for WeightedRoundRobin {
    fn next_server(&self) -> SocketAddr {
        self.cycle.next_as_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_round_robin() {
        let backends = vec![
            ("127.0.0.1:8080", 1),
            ("127.0.0.1:8081", 3),
            ("127.0.0.1:8082", 2),
        ];

        let expected = vec![
            "127.0.0.1:8080",
            "127.0.0.1:8081",
            "127.0.0.1:8081",
            "127.0.0.1:8081",
            "127.0.0.1:8082",
            "127.0.0.1:8082",
        ];

        let wrr = WeightedRoundRobin::new(
            &backends
                .iter()
                .map(|(addr, weight)| Backend {
                    address: addr.parse().unwrap(),
                    weight: *weight,
                })
                .collect(),
        );

        for server in expected {
            assert_eq!(server, wrr.next_server().to_string());
        }
    }
}
