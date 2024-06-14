use hyper::{Body, Request as HyperRequest, Response, Server as HyperServer, Method};
use hyper::service::{make_service_fn, service_fn};
use std::sync::Arc;
use std::net::SocketAddr;

mod handlers;
mod http;
mod request;
mod routing;

use http::HttpMethod;
use request::Request;
use routing::{Route, Router};

use crate::threading::LoadBalancer;

const DIR_FLAG: &str = "--directory";

pub struct AppState {
    router: Arc<Router>,
    cfg: Arc<Cfg>,
    pool: LoadBalancer,
}

struct Cfg {
    files_dir: Option<String>,
}

pub struct Server {
    addr: SocketAddr,
    state: Arc<AppState>,
}

impl Server {
    pub fn setup(addr: &str, pool_size: usize, args: &[String]) -> Server {
        let addr = addr.parse().expect("Unable to parse socket address");
        let router = Arc::new(setup_router());
        let cfg = Arc::new(setup_cfg(&args));
        let pool = LoadBalancer::new(pool_size);

        let state = Arc::new(AppState { router, cfg, pool });

        Server { addr, state }
    }

    pub async fn run(&self) {
        let make_svc = make_service_fn(move |_conn| {
            let state = Arc::clone(&self.state);
            async {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    Server::handle_connection(Arc::clone(&state), req)
                }))
            }
        });

        let server = HyperServer::bind(&self.addr).serve(make_svc);

        println!("Listening on http://{}", self.addr);

        if let Err(e) = server.await {
            eprintln!("Server error: {}", e);
        }
    }

    async fn handle_connection(
        state: Arc<AppState>,
        req: HyperRequest<Body>,
    ) -> Result<Response<Body>, hyper::Error> {
        let (parts, body) = req.into_parts();
        let body_bytes = hyper::body::to_bytes(body).await?;
        let stream = &body_bytes[..];

        // Log the raw buffer, the request method and URI
        println!("Received request: {} {}", parts.method, parts.uri);
        println!("Raw buffer: {:?}", stream);

        let request = match request::Request::parse(stream) {
            Ok(request) => request,
            Err(err) => {
                println!("Problem parsing request: {}", err);
                let response = Response::builder()
                    .status(400)
                    .body(Body::from("Bad Request"))
                    .unwrap();
                return Ok(response);
            },
        };

        let handler = state.router.get_handler(&request);
        let cfg = Arc::clone(&state.cfg);

        let response = handler(&cfg, &request);
        Ok(Response::new(Body::from(response)))
    }
}

fn setup_cfg(args: &[String]) -> Cfg {
    let files_dir = setup_directory(&args);
    Cfg { files_dir }
}

fn setup_router() -> Router {
    Router::new(vec![
        Route::new("/", HttpMethod::GET, handlers::handle_200),
        Route::new("/echo/", HttpMethod::GET, handlers::handle_echo),
        Route::new("/user-agent", HttpMethod::GET, handlers::handle_user_agent),
        Route::new("/files/", HttpMethod::GET, handlers::handle_get_file),
        Route::new("/files/", HttpMethod::POST, handlers::handle_post_file),
    ])
}

fn setup_directory(args: &[String]) -> Option<String> {
    let dir_flag_index = args.iter().position(|arg| arg == DIR_FLAG);

    match dir_flag_index {
        Some(dir_flag_index) => {
            let path = args.get(dir_flag_index + 1);
            match path {
                Some(path) => {
                    let path = path.clone();
                    std::fs::create_dir_all(&path)
                        .expect(&format!("Can't create directory at {}", &path));
                    return Some(path);
                },
                None => {
                    panic!("No `directory` argument provided for --directory")
                }
            };
        },
        None => None,
    }
}
