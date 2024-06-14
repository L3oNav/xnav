use std::fs;

use super::{
    Cfg,
    request::Request,
    http::HTTPStatus,
};

pub type Handler = fn (&Cfg, &Request) -> String;

pub fn handle_404(_: &Cfg, _: &Request) -> String {
    format!("{}\r\n\r\n", HTTPStatus::NotFound.as_str())
}

pub fn handle_200(_: &Cfg, _: &Request) -> String {
    format!(
        "{}\r\n\r\n",
        HTTPStatus::Ok.as_str()
    )
}

pub fn handle_echo(_: &Cfg, request: &Request) -> String {
    let split: Vec<&str> = request.path.path
        .split('/')
        .filter(|item| !item.is_empty())
        .collect();

    let contents = split[1..].join("/");

    format!(
        "{}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        HTTPStatus::Ok.as_str(),
        contents.len(),
        contents
    )
}

pub fn handle_user_agent(_: &Cfg, request: &Request) -> String {
    let contents = request.headers
        .get("User-Agent")
        .map_or("Unknown", String::as_str);

    format!(
        "{}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
        HTTPStatus::Ok.as_str(),
        contents.len(),
        contents
    )
}

pub fn handle_get_file(cfg: &Cfg, request: &Request) -> String {
    match &cfg.files_dir {
        Some(file_dir) => {
            let filename = request.path.path["/files/".len()..].trim();
            // naive
            let file_path = format!("{file_dir}/{filename}");
            let contents = fs::read_to_string(file_path);

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
}

pub fn handle_post_file(cfg: &Cfg, request: &Request) -> String {
    match &cfg.files_dir {
        Some(file_dir) => {
            let filename = request.path.path["/files/".len()..].trim();
            let file_path = format!("{file_dir}/{filename}");

            match fs::write(file_path, &request.body) {
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
}
