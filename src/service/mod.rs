//! Proxy server module, handling HTTP requests, serving static files, and proxying to backend servers.

mod body;
mod files;
mod proxy;

pub mod request;
pub mod response;

pub use body::{empty, full};
pub use files::transfer;
pub use proxy::forward;
pub use request::ProxyRequest;
pub use response::{BoxBodyResponse, LocalResponse, ProxyResponse};

use crate::config::{self, Action, Forward};
use hyper::{body::Incoming, service::Service, Request};
use tokio::time::Instant;

use std::{future::Future, net::SocketAddr, pin::Pin};

pub struct Xnav {
    config: &'static config::Server,
    client_addr: SocketAddr,
    server_addr: SocketAddr,
}

impl Xnav {
    /// Creates a new [`Xnav`] service.
    pub fn new(
        config: &'static config::Server,
        client_addr: SocketAddr,
        server_addr: SocketAddr,
    ) -> Self {
        Self {
            config,
            client_addr,
            server_addr,
        }
    }
}

impl Service<Request<Incoming>> for Xnav {
    type Response = BoxBodyResponse;

    type Error = hyper::Error;

    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn call(&self, request: Request<Incoming>) -> Self::Future {
        let Xnav {
            client_addr,
            server_addr,
            config,
        } = *self;

        let instant = Instant::now();

        Box::pin(async move {
            let uri = request.uri().to_string();
            let method = request.method().to_string();

            let maybe_pattern = config
                .patterns
                .iter()
                .find(|pattern| uri.starts_with(pattern.uri.as_str()));

            let Some(pattern) = maybe_pattern else {
                return Ok(LocalResponse::not_found());
            };

            let response = match &pattern.action {
                Action::Forward(Forward { scheduler, .. }) => {
                    let by = config.name.as_ref().map(|name| name.clone());
                    let request = ProxyRequest::new(request, client_addr, server_addr, by);
                    proxy::forward(request, scheduler.next_server()).await
                }

                Action::Serve(directory) => {
                    let path = if request.uri().path().starts_with("/") {
                        &request.uri().path()[1..]
                    } else {
                        request.uri().path()
                    };
                    files::transfer(path, directory).await
                }
            };

            if let Ok(response) = &response {
                let status = response.status();
                let log_name = &config.log_name;
                let elapsed = instant.elapsed();
                println!("{client_addr} -> {log_name} {method} {uri} HTTP {status} {elapsed:?}");
            }

            response
        })
    }
}
