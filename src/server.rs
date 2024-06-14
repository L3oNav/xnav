use hyper::{Body, Request, Response, Server as HyperServer, Method};
use hyper::service::{make_service_fn, service_fn};
use std::sync::Arc;
use std::net::SocketAddr;
use futures::future::BoxFuture;

mod handlers;
mod http;
mod routing;

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
        req: Request<Body>,
    ) -> Result<Response<Body>, hyper::Error> {
        println!("Received request: {} {}", req.method(), req.uri());

        let handler = state.router.get_handler(&req);
        let cfg = Arc::clone(&state.cfg);

        let response = handler(&cfg, req).await;
        Ok(Response::new(Body::from(response)))
    }
}

fn setup_cfg(args: &[String]) -> Cfg {
    let files_dir = setup_directory(&args);
    Cfg { files_dir }
}

fn setup_router() -> Router {
    Router::new(vec![
        Route::new("/", Method::GET, handlers::handle_200),
        Route::new("/echo/", Method::GET, handlers::handle_echo),
        Route::new("/user-agent", Method::GET, handlers::handle_user_agent),
        Route::new("/files/", Method::GET, handlers::handle_get_file),
        Route::new("/files/", Method::POST, handlers::handle_post_file),
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
                    if let Err(e) = std::fs::create_dir_all(&path) {
                        eprintln!("Can't create directory at {}: {}", &path, e);
                        std::process::exit(1);
                    }
                    return Some(path);
                },
                None => {
                    eprintln!("No `directory` argument provided for --directory");
                    std::process::exit(1);
                }
            };
        },
        None => None,
    }
}
