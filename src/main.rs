use data_rs::server::run_server;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use tokio;

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_colors(true)
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    run_server().await;
}
