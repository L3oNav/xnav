//! Types and abstractions for HTTP responses.

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use hyper::{
    header::{self, HeaderValue},
    Response,
};

pub type BoxBodyResponse = Response<BoxBody<Bytes, hyper::Error>>;

/// Response sent back to the client at the end of the proxying process.
pub struct ProxyResponse<T> {
    response: Response<T>,
}

impl<T> ProxyResponse<T> {
    pub fn new(response: Response<T>) -> Self {
        Self { response }
    }

    pub fn into_forwarded(mut self) -> Response<T> {
        self.response.headers_mut().insert(
            header::SERVER,
            HeaderValue::from_str(crate::rxh_server_header().as_str()).unwrap(),
        );
        self.response
    }
}

/// HTTP response originated on this server.
pub struct LocalResponse;

impl LocalResponse {
    pub fn builder() -> http::response::Builder {
        Response::builder().header(header::SERVER, crate::rxh_server_header())
    }

    pub fn not_found() -> BoxBodyResponse {
        Self::builder()
            .status(http::StatusCode::NOT_FOUND)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(crate::full("HTTP 404 NOT FOUND"))
            .unwrap()
    }

    pub fn bad_gateway() -> BoxBodyResponse {
        Self::builder()
            .status(http::StatusCode::BAD_GATEWAY)
            .header(header::CONTENT_TYPE, "text/plain")
            .body(crate::full("HTTP 502 BAD GATEWAY"))
            .unwrap()
    }
}

pub fn rxh_server_header() -> String {
    format!("rxh/{}", crate::VERSION)
}
