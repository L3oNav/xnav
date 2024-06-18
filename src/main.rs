#[tokio::main]
async fn main() -> Result<(), xnav::Error> {
    let config = toml::from_str(&tokio::fs::read_to_string("config.toml").await?)?;
    xnav::Master::init(config)?
        .shutdown_on(tokio::signal::ctrl_c())
        .run()
        .await
}
