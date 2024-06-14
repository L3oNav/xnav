use anyhow::{anyhow, Error};
use std::collections::HashMap;
use std::io::{prelude::*, BufReader};

use super::http::HttpMethod;

#[derive(Debug)]
pub struct Path {
    pub path: String,
    pub query: String,
}

impl Path {
    pub fn build(path: String) -> Path {
        let mut split = path.split('?');
        let path = split.next().unwrap_or_default().to_owned();
        let query = split.next().unwrap_or_default().to_owned();
        Path { path, query }
    }
}

#[derive(Debug)]
pub struct Request {
    pub method: HttpMethod,
    pub path: Path,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl Request {
    pub fn parse(buf: &[u8]) -> Result<Request, Error> {
        let mut buf_reader = BufReader::new(buf);

        let (method, path) = Request::parse_status_line(&mut buf_reader)?;
        let headers = Request::parse_headers(&mut buf_reader)?;

        // Safely get Content-Length as a String
        let content_length = headers.get("Content-Length").unwrap_or(&"0".to_string()).to_string();
        let body = Request::parse_body(&mut buf_reader, &content_length)?;

        Ok(Request { method, path, headers, body })
    }

    fn parse_status_line<R: BufRead>(buf_reader: &mut R) -> Result<(HttpMethod, Path), Error> {
        let mut status_line = String::new();
        buf_reader.read_line(&mut status_line)?;
        
        // Log the status line for debugging
        println!("Status Line: '{}'", status_line.trim());

        if status_line.trim().is_empty() {
            return Err(anyhow!("Received empty status line"));
        }

        let mut parts = status_line.splitn(3, ' ');
        let method = HttpMethod::parse(parts.next().unwrap_or_default());
        let path = Path::build(parts.next().unwrap_or_default().to_owned());

        Ok((method, path))
    }

    fn parse_headers<R: BufRead>(buf_reader: &mut R) -> Result<HashMap<String, String>, Error> {
        let mut headers = HashMap::new();
        let mut header_line = String::new();

        loop {
            buf_reader.read_line(&mut header_line)?;
            
            if header_line == "\r\n" {
                break;
            }
            
            // Log each header line for debugging
            println!("Header Line: '{}'", header_line.trim());

            if let Some((key, val)) = header_line.split_once(':') {
                headers.insert(
                    clean_header_value(key),
                    clean_header_value(val),
                );
                header_line.clear();
            } else {
                // Provide more context in the error message
                return Err(anyhow!("Invalid header line: '{}'", header_line.trim()));
            }
        }

        Ok(headers)
    }

    fn parse_body<R: BufRead>(buf_reader: &mut R, content_len: &str) -> Result<String, Error> {
        let len = content_len.parse::<u64>().map_err(|_| anyhow!("Invalid Content-Length value"))?;
        
        if len > 0 {
            let mut buf = buf_reader.take(len);
            let mut body = vec![];
            buf.read_to_end(&mut body)?;
            Ok(String::from_utf8_lossy(&body).to_string())
        } else {
            Ok(String::new())
        }
    }
}

fn clean_header_value(val: &str) -> String {
    val.trim().to_owned()
}
