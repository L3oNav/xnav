use super::handlers::*;
use super::http::HttpMethod;
use super::request::Request;

pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    pub fn new(routes: Vec<Route>) -> Router {
        Router{ routes }
    }

    pub fn get_handler(&self, request: &Request) -> Handler {
        let opt_route = self.routes
            .iter()
            .find(|&route| 
                route.method == request.method 
                && route.matches(&request.path.path)
            );

        match opt_route {
            Some(route) => route.handler,
            _ => handle_404
        }
    }
}

pub struct Route {
    path: String,
    method: HttpMethod,
    handler: Handler,
}

impl Route {
    pub fn new(path: &str, method: HttpMethod, handler: Handler) -> Route {
        Route {
            path: path.to_string(), 
            method, 
            handler
        }
    }

    pub fn matches(&self, path: &String) -> bool {
        // TODO use regexp to match

        // root case
        if self.path.len() == 1 {
            return &self.path == path
        }

        path.starts_with(&self.path)
    }
}
