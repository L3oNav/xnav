use std::{future::Future, io, net::SocketAddr, pin::Pin, ptr, sync::Arc};

use hyper::server::conn::http1::Builder;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{watch, Semaphore},
    TcpSocket,
};
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::{
    config,
    service::Xnav,
    sync::{Notification, Notifier},
};
pub struct Server {
    state: watch::Sender<State>,
    listener: TcpListener,
    config: config::Server,
    address: SocketAddr,
    notifier: Notifier,
    shutdown: Pin<Box<dyn Future<Output = ()> + Send>>,
    connections: Arc<Semaphore>,
}

/// Represents the current state of the server.
#[derive(Debug, PartialEq, Eq)]
pub enum State {
    Starting,
    Listening,
    MaxConnectionsReached(usize),
    ShuttingDown(ShutdownState),
}

/// Represents a state in the graceful shutdown process.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ShutdownState {
    PendingConnections(usize),
    Done,
}

impl Server {
    /// Initializes a server with the given configuration.
    pub fn init(config: config::Server, replica: usize) -> Result<Self, io::Error> {
        let (state, _) = watch::channel(State::Starting);

        let socket = if config.listen[replica].is_ipv4() {
            TcpSocket::new_v4()?
        } else {
            TcpSocket::new_v6()?
        };

        #[cfg(not(windows))]
        socket.set_reuseaddr(true)?;

        socket.bind(config.listen[replica])?;
        let listener = socket.listen(1024)?;
        let address = listener.local_addr().unwrap();
        let notifier = Notifier::new();
        let shutdown = Box::pin(std::future::pending());
        let connections = Arc::new(Semaphore::new(config.max_connections));

        Ok(Self {
            state,
            listener,
            config,
            address,
            notifier,
            shutdown,
            connections,
        })
    }

    /// Sets a termination future for server shutdown.
    pub fn shutdown_on(mut self, future: impl Future + Send + 'static) -> Self {
        self.shutdown = Box::pin(async move {
            future.await;
        });
        self
    }

    /// Gets the socket address of the listener.
    pub fn socket_address(&self) -> SocketAddr {
        self.address
    }

    /// Subscribes to server state updates.
    pub fn subscribe(&self) -> watch::Receiver<State> {
        self.state.subscribe()
    }

    /// Begins accepting connections and running the server.
    pub async fn run(self) -> Result<(), crate::Error> {
        let Self {
            mut config,
            state,
            listener,
            notifier,
            shutdown,
            address,
            connections,
        } = self;

        let log_name = if let Some(ref id) = config.name {
            format!("{address} ({id})")
        } else {
            address.to_string()
        };

        config.log_name = log_name.clone();

        state.send_replace(State::Listening);
        println!("{log_name} => Listening for requests");

        let config = Box::leak(Box::new(config));

        let listener = Listener {
            config,
            connections,
            listener,
            notifier: &notifier,
            state: &state,
        };

        tokio::select! {
            result = listener.listen() => {
                if let Err(err) = result {
                    println!("{log_name} => Error while accepting connections: {err}");
                }
            }
            _ = shutdown => {
                println!("{log_name} => Received shutdown signal");
            }
        }

        drop(listener);

        if let Ok(num_tasks) = notifier.send(Notification::Shutdown) {
            println!("{log_name} => Can't shutdown yet, {num_tasks} pending connections");
            state.send_replace(State::ShuttingDown(ShutdownState::PendingConnections(
                num_tasks,
            )));
            notifier.collect_acknowledgements().await;
        }

        unsafe {
            drop(Box::from_raw(ptr::from_ref(config).cast_mut()));
        }

        state.send_replace(State::ShuttingDown(ShutdownState::Done));
        println!("{log_name} => Shutdown complete");

        Ok(())
    }
}

struct Listener<'a> {
    listener: TcpListener,
    config: &'static config::Server,
    notifier: &'a Notifier,
    state: &'a watch::Sender<State>,
    connections: Arc<Semaphore>,
}

impl<'a> Listener<'a> {
    pub async fn listen(&self) -> Result<(), crate::Error> {
        loop {
            let config = self.config;
            let mut notify_listening_again = false;

            if self.connections.available_permits() == 0 {
                println!(
                    "{} => Reached max connections: {}",
                    config.log_name, config.max_connections
                );
                self.state
                    .send_replace(State::MaxConnectionsReached(config.max_connections));
                notify_listening_again = true;
            }

            let permit = self.connections.clone().acquire_owned().await.unwrap();

            if notify_listening_again {
                println!("{} => Accepting connections again", config.log_name);
                self.state.send_replace(State::Listening);
            }

            let (stream, client_addr) = self.listener.accept().await?;
            let mut subscription = self.notifier.subscribe();
            let server_addr = stream.local_addr()?;

            tokio::task::spawn(async move {
                if let Err(err) = Builder::new()
                    .preserve_header_case(true)
                    .title_case_headers(true)
                    .serve_connection(stream, Xnav::new(config, client_addr, server_addr))
                    .with_upgrades()
                    .await
                {
                    println!("Failed to serve connection: {:?}", err);
                }

                if let Some(Notification::Shutdown) = subscription.receive_notification() {
                    subscription.acknowledge_notification().await;
                }

                drop(permit);
            });
        }
    }
}
