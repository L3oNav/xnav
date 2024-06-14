use super::handlers::*;
use hyper::{Request, Body, Method};
use futures::future::BoxFuture;

pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    pub fn new(routes: Vec<Route>) -> Router {
        Router { routes }
    }

    pub fn get_handler(&self, req: &Request<Body>) -> Handler {
        let path = req.uri().path();
        let method = req.method();

        let opt_route = self.routes
            .iter()
            .find(|&route| 
                &route.method == method 
                && route.matches(path)
            );

        match opt_route {
            Some(route) => route.handler,
            _ => handle_404
        }
    }
}

pub struct Route {
    path: String,
    method: Method,
    handler: Handler,
}

impl Route {
    pub fn new(path: &str, method: Method, handler: Handler) -> Route {
        Route {
            path: path.to_string(), 
            method, 
            handler
        }
    }

    pub fn matches(&self, path: &str) -> bool {
        if self.path == "/" {
            return self.path == path;
        }

        path.starts_with(&self.path)
    }
}
