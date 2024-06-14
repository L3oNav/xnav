use std::env;

mod server;
mod threading;

use server::Server;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    let server = Server::setup(
        "127.0.0.1:3312",
        4,
        &args,
    );

    server.run().await;
}
