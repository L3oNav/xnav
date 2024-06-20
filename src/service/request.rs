use http::{Extensions, HeaderMap, Uri};
use hyper::{header, upgrade::OnUpgrade, Request};
use std::net::SocketAddr;

/// Request received by this proxy from a client.
pub struct ProxyRequest<T> {
    request: Request<T>,
    client_addr: SocketAddr,
    server_addr: SocketAddr,
    proxy_id: Option<String>,
}

impl<T> ProxyRequest<T> {
    pub fn new(
        request: Request<T>,
        client_addr: SocketAddr,
        server_addr: SocketAddr,
        proxy_id: Option<String>,
    ) -> Self {
        Self {
            request,
            client_addr,
            server_addr,
            proxy_id,
        }
    }

    pub fn headers(&self) -> &HeaderMap {
        self.request.headers()
    }

    pub fn extensions_mut(&mut self) -> &mut Extensions {
        self.request.extensions_mut()
    }

    pub fn into_forwarded(mut self) -> Request<T> {
        let host = if let Some(value) = self.request.headers().get(header::HOST) {
            match value.to_str() {
                Ok(host) => String::from(host),
                Err(_) => self.server_addr.to_string(),
            }
        } else {
            self.server_addr.to_string()
        };

        let by = self.proxy_id.unwrap_or(self.server_addr.to_string());

        let mut forwarded = format!("for={};by={};host={}", self.client_addr, by, host);

        if let Some(value) = self.request.headers().get(header::FORWARDED) {
            if let Ok(previous_proxies) = value.to_str() {
                forwarded = format!("{previous_proxies}, {forwarded}");
            }
        }

        self.request.headers_mut().insert(
            header::FORWARDED,
            header::HeaderValue::from_str(&forwarded).unwrap(),
        );

        self.request
    }

    pub fn uri(&self) -> &Uri {
        self.request.uri()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hyper::Body;

    fn forwarded_header<T>(request: &Request<T>) -> String {
        let forwarded = request
            .headers()
            .get(header::FORWARDED)
            .unwrap()
            .to_str()
            .unwrap();

        String::from(forwarded)
    }

    #[test]
    fn forwarded_request() {
        let client = "127.0.0.1:8000".parse().unwrap();
        let proxy = "127.0.0.1:9000".parse().unwrap();

        let request = ProxyRequest::new(
            Request::builder().body(Body::empty()).unwrap(),
            client,
            proxy,
            None,
        );

        let forwarded = request.into_forwarded();
        let expected = format!("for={client};by={proxy};host={proxy}");

        assert!(forwarded.headers().contains_key(header::FORWARDED));
        assert_eq!(forwarded_header(&forwarded), expected.as_str());
    }

    #[test]
    fn forwarded_request_with_proxy_id() {
        let client = "127.0.0.1:8000".parse().unwrap();
        let proxy = "127.0.0.1:9000".parse().unwrap();
        let proxy_id = String::from("rxh/main");

        let request = ProxyRequest::new(
            Request::builder().body(Body::empty()).unwrap(),
            client,
            proxy,
            Some(proxy_id.clone()),
        );

        let forwarded = request.into_forwarded();
        let expected = format!("for={client};by={proxy_id};host={proxy}");

        assert!(forwarded.headers().contains_key(header::FORWARDED));
        assert_eq!(forwarded_header(&forwarded), expected.as_str());
    }
}
