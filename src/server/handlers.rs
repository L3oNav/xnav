use hyper::{Body, Request};
use super::Cfg;
use super::http::HTTPStatus;
use futures::future::BoxFuture;

pub type Handler = fn(&Cfg, Request<Body>) -> BoxFuture<'static, String>;

pub fn handle_404(_: &Cfg, _: Request<Body>) -> BoxFuture<'static, String> {
    Box::pin(async { format!("{}\r\n\r\n", HTTPStatus::NotFound.as_str()) })
}

pub fn handle_200(_: &Cfg, _: Request<Body>) -> BoxFuture<'static, String> {
    Box::pin(async { format!("{}\r\n\r\n", HTTPStatus::Ok.as_str()) })
}

pub fn handle_echo(_: &Cfg, req: Request<Body>) -> BoxFuture<'static, String> {
    let path = req.uri().path().trim_start_matches("/echo/").to_string();
    Box::pin(async move {
        format!(
            "{}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            HTTPStatus::Ok.as_str(),
            path.len(),
            path
        )
    })
}

pub fn handle_user_agent(_: &Cfg, req: Request<Body>) -> BoxFuture<'static, String> {
    let user_agent = req
        .headers()
        .get("User-Agent")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("Unknown")
        .to_string();

    Box::pin(async move {
        format!(
            "{}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
            HTTPStatus::Ok.as_str(),
            user_agent.len(),
            user_agent
        )
    })
}

pub fn handle_get_file(cfg: &Cfg, req: Request<Body>) -> BoxFuture<'static, String> {
    let path = req.uri().path().trim_start_matches("/files/").to_string();
    let files_dir = cfg.files_dir.clone();
    Box::pin(async move {
        match &files_dir {
            Some(file_dir) => {
                let file_path = format!("{}/{}", file_dir, path);
                let contents = std::fs::read_to_string(file_path);

                match contents {
                    Ok(contents) => {
                        format!(
                            "{}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n{}",
                            HTTPStatus::Ok.as_str(),
                            contents.len(),
                            contents
                        )
                    },
                    Err(_) => format!("{}\r\n\r\n", HTTPStatus::NotFound.as_str()),
                }            
            },
            None => format!("{}\r\n\r\n", HTTPStatus::NotFound.as_str()),
        }
    })
}

pub fn handle_post_file(cfg: &Cfg, req: Request<Body>) -> BoxFuture<'static, String> {
    let path = req.uri().path().trim_start_matches("/files/").to_string();
    let files_dir = cfg.files_dir.clone();
    Box::pin(async move {
        let body = hyper::body::to_bytes(req.into_body()).await.unwrap();
        match &files_dir {
            Some(file_dir) => {
                let file_path = format!("{}/{}", file_dir, path);
    
                match std::fs::write(file_path, body) {
                    Ok(_) => {
                        format!(
                            "{}\r\nContent-Type: text/plain\r\nContent-Length: 0\r\n\r\n",
                            HTTPStatus::Created.as_str(),
                        )
                    },
                    Err(_) => {
                        format!("{}\r\n\r\n", HTTPStatus::InternalServerError.as_str())
                    },
                }
            },
            None => format!("{}\r\n\r\n", HTTPStatus::NotFound.as_str()),
        }
    })
}
