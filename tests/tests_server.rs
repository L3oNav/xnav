use hyper::{Body, Client, Method, Request, Uri};
use std::sync::Arc;
use std::error::Error;
use xnav::server::Server;

async fn start_server(address: &str, args: Vec<String>) -> Arc<Server> {
    let server = Server::setup(address, 4, &args);
    let server_instance = Arc::new(server);

    let server_clone: Arc<Server> = Arc::clone(&server_instance);
    tokio::spawn(async move {
        server_clone.run().await;
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    server_instance
}

#[tokio::test]
async fn test_server_setup() -> Result<(), Box<dyn Error>> {
    let address = "127.0.0.1:3312";
    let args = vec!["server".into(), "--directory".into(), "test_dir".into()];
    start_server(&address, args).await;

    let client = Client::new();
    let uri: Uri = format!("http://{}/", address).parse()?;
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())?;

    let res = client.request(req).await?;
    assert_eq!(res.status(), 200);
    Ok(())
}

#[tokio::test]
async fn test_server_handle_echo() -> Result<(), Box<dyn Error>> {
    let port = "3312";
    let address = format!("127.0.0.1:{}", port);
    let args = vec!["server".into(), "--directory".into(), "test_dir".into()];
    start_server(&address, args).await;

    let client = Client::new();
    let uri: Uri = format!("http://{}/echo/EchoTest", address).parse()?;
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())?;

    let res = client.request(req).await?;
    let body_bytes = hyper::body::to_bytes(res.into_body()).await?;
    let body_str = std::str::from_utf8(&body_bytes)?;

    assert_eq!(body_str, "EchoTest");
    Ok(())
}
