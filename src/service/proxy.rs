use std::net::SocketAddr;

use http_body_util::BodyExt;
use hyper::{
    body::{Body, Incoming},
    client::conn::http1::Builder,
    header,
    upgrade::{OnUpgrade, Upgraded},
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_util::compat::TokioAsyncReadCompatExt;

use crate::service::{
    request::ProxyRequest,
    response::{BoxBodyResponse, LocalResponse, ProxyResponse},
};

pub(super) async fn forward(
    mut request: ProxyRequest<Incoming>,
    to: SocketAddr,
) -> Result<BoxBodyResponse, hyper::Error> {
    let Ok(stream) = TcpStream::connect(to).await else {
        return Ok(LocalResponse::bad_gateway());
    };

    let stream = stream.compat(); // Convert into a compatible type

    let (mut sender, conn) = Builder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .handshake(stream)
        .await?;

    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            println!("Connection failed: {:?}", err);
        }
    });

    let mut maybe_client_upgrade = None;

    if request.headers().contains_key(header::UPGRADE) {
        let upgrade = request.extensions_mut().remove::<OnUpgrade>().unwrap();
        maybe_client_upgrade = Some(upgrade);
    }

    let mut response = sender.send_request(request.into_forwarded()).await?;

    if response.status() == http::StatusCode::SWITCHING_PROTOCOLS {
        if let Some(client_upgrade) = maybe_client_upgrade {
            let server_upgrade = response.extensions_mut().remove::<OnUpgrade>().unwrap();
            tokio::task::spawn(tunnel(client_upgrade, server_upgrade));
        } else {
            return Ok(LocalResponse::bad_gateway());
        }
    }

    Ok(ProxyResponse::new(response.map(|body| body.boxed())).into_forwarded())
}

async fn tunnel(client: OnUpgrade, server: OnUpgrade) {
    let (mut upgraded_client, mut upgraded_server) = tokio::try_join!(client, server).unwrap();

    match tokio::io::copy_bidirectional(&mut upgraded_client, &mut upgraded_server).await {
        Ok((client_bytes, server_bytes)) => {
            println!("Client wrote {client_bytes} bytes, server wrote {server_bytes} bytes")
        }
        Err(err) => eprintln!("Tunnel error: {err}"),
    }
}
