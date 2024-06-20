use std::future::{self, Future};
use std::net::SocketAddr;
use std::pin::Pin;

use tokio::sync::{broadcast, watch};

use crate::{
    config::Config,
    server::{Server, State},
};

/// The master task is responsible for creating, spawning, and shutting down all the server instances described in the configuration file.
pub struct Master {
    servers: Vec<Server>,
    states: Vec<(SocketAddr, watch::Receiver<State>)>,
    shutdown: Pin<Box<dyn Future<Output = ()> + Send>>,
    shutdown_notify: broadcast::Sender<()>,
}

impl Master {
    /// Attempts to initialize all the servers specified in the configuration file.
    pub fn init(config: Config) -> Result<Self, crate::Error> {
        let mut servers = Vec::new();
        let mut states = Vec::new();
        let shutdown = Box::pin(future::pending());
        let (shutdown_notify, _) = broadcast::channel(1);

        for server_config in config.servers {
            for replica in 0..server_config.listen.len() {
                let server = Server::init(server_config.clone(), replica)?;
                states.push((server.socket_address(), server.subscribe()));
                servers.push(server);
            }
        }

        Ok(Self {
            servers,
            states,
            shutdown,
            shutdown_notify,
        })
    }

    /// Sets a future to initiate termination when `future` completes.
    pub fn shutdown_on(mut self, future: impl Future + Send + 'static) -> Self {
        self.servers = self
            .servers
            .into_iter()
            .map(|server| {
                let mut shutdown_notification = self.shutdown_notify.subscribe();
                server.shutdown_on(async move { shutdown_notification.recv().await })
            })
            .collect();

        self.shutdown = Box::pin(async move {
            future.await;
        });

        self
    }

    /// Runs all servers and initiates termination when the shutdown future completes.
    pub async fn run(self) -> Result<(), crate::Error> {
        let mut set = tokio::task::JoinSet::new();

        for server in self.servers {
            set.spawn(server.run());
        }

        let mut first_error = None;

        tokio::select! {
            Some(Ok(Err(err))) = set.join_next() => {
                first_error = Some(err);
                println!("Master => Received error while waiting for shutdown");
            }

            _ = self.shutdown => {
                println!("Master => Sending shutdown signal to all servers");
            }
        }

        self.shutdown_notify.send(()).unwrap();

        while let Some(result) = set.join_next().await {
            if let Err(err) = result.unwrap() {
                first_error.get_or_insert(err);
            }
        }

        match first_error {
            None => Ok(()),
            Some(err) => Err(crate::Error::from(err)),
        }
    }

    /// Returns the addresses of all listening sockets.
    pub fn sockets(&self) -> Vec<SocketAddr> {
        self.states.iter().map(|(addr, _)| *addr).collect()
    }
}
