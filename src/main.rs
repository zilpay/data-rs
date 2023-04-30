use data_rs::{
    models::{currencies::Currencies, dex::Dex, meta::Meta},
    server::run_server,
    utils::zilliqa::Zilliqa,
};
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::{sync::Arc, time::Duration};
use tokio;
use tokio::sync::RwLock;

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_colors(true)
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let meta = Arc::new(RwLock::new(Meta::new()));
    let rates = Arc::new(RwLock::new(Currencies::new()));
    let dex = Arc::new(RwLock::new(Dex::new()));

    let meta_ref = Arc::clone(&meta);
    let dex_ref = Arc::clone(&dex);
    let rates_ref = Arc::clone(&rates);

    tokio::task::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            let zil = Zilliqa::new();
            let mut meta = meta_ref.write().await;

            meta.update(&zil).await.unwrap();

            let dex = dex_ref.read().await;

            meta.listed_tokens_update(&dex);
        }
    });

    tokio::task::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            let mut rates = rates_ref.write().await;

            rates.update().await.unwrap();
        }
    });

    tokio::task::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            let zil = Zilliqa::new();
            let mut dex = dex_ref.write().await;

            dex.update(&zil).await.unwrap();
        }
    });

    let meta_ref0 = Arc::clone(&meta);
    let dex_ref0 = Arc::clone(&dex);
    let rates_ref0 = Arc::clone(&rates);

    run_server(&meta_ref0, &dex_ref0, &rates_ref0)
        .await
        .unwrap();
}
