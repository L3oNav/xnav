//! Proxy server module, handling HTTP requests, serving static files, and proxying to backend servers.

mod body;
mod files;
mod proxy;
mod request;
mod response;

pub use body::{empty, full};
pub use files::transfer;
pub use proxy::forward;
pub use request::ProxyRequest;
pub use response::{BoxBodyResponse, LocalResponse, ProxyResponse};
