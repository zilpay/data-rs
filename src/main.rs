use data_rs::{
    models::{currencies::Currencies, dex::Dex, meta::Meta},
    server::run_server,
    utils::zilliqa::Zilliqa,
};
use log::{error, LevelFilter};
use simple_logger::SimpleLogger;
use std::time::Duration;
use tokio;

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_colors(true)
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();
    let mut meta = Meta::new();
    let mut rates = Currencies::new();
    let mut dex = Dex::new();

    tokio::task::spawn(async move {
        let zil = Zilliqa::new();

        loop {
            match meta.update(&zil).await {
                Ok(_) => (),
                Err(e) => error!("{:?}", e),
            };
            match dex.update(&zil).await {
                Ok(_) => (),
                Err(e) => error!("{:?}", e),
            };
            match rates.update().await {
                Ok(_) => (),
                Err(e) => error!("{:?}", e),
            };

            meta.listed_tokens_update(&dex);

            tokio::time::sleep(Duration::from_secs(300)).await;
        }
    });

    run_server().await.unwrap();
}
