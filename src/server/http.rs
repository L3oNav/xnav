#[derive(Eq, Hash, PartialEq, Debug)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,

    UNKNOWN,
}

impl HttpMethod {
    pub fn parse(method: &str) -> HttpMethod {
        match method {
            "GET" => HttpMethod::GET,
            "POST" => HttpMethod::POST,
            "PUT" => HttpMethod::PUT,
            _ => HttpMethod::UNKNOWN,
        }
    }
}

pub enum HTTPStatus {
    Ok = 200,
    Created = 201,
    NotFound = 404,
    InternalServerError = 500,
    // BadRequest = 400,
}

impl HTTPStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            HTTPStatus::Ok => "HTTP/1.1 200 OK",
            HTTPStatus::Created => "HTTP/1.1 201 Created",
            HTTPStatus::NotFound => "HTTP/1.1 404 Not Found",
            HTTPStatus::InternalServerError => "HTTP/1.1 500 Internal Server Error",
            // HTTPStatus::BadRequest => "HTTP/1.1 400 Internal Server Error",
        }
    }
}
