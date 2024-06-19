//! Static files server sub-service.

use crate::response::{BoxBodyResponse, LocalResponse};
use hyper::header;
use std::path::Path;

/// Returns an HTTP response whose body is the content of a file.
pub async fn transfer(path: &str, root: &str) -> Result<BoxBodyResponse, hyper::Error> {
    let Ok(directory) = Path::new(root).canonicalize() else {
        return Ok(LocalResponse::not_found());
    };

    let Ok(file) = directory.join(path).canonicalize() else {
        return Ok(LocalResponse::not_found());
    };

    if !file.starts_with(&directory) || !file.is_file() {
        return Ok(LocalResponse::not_found());
    }

    let content_type = match file.extension().and_then(|e| e.to_str()).unwrap_or("txt") {
        "html" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "png" => "image/png",
        "jpeg" => "image/jpeg",
        _ => "text/plain",
    };

    match tokio::fs::read(file).await {
        Ok(content) => Ok(LocalResponse::builder()
            .header(header::CONTENT_TYPE, content_type)
            .body(crate::full(content))
            .unwrap()),
        Err(_) => Ok(LocalResponse::not_found()),
    }
}
