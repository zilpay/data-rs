use data_rs::{
    models::{currencies::Currencies, dex::Dex, meta::Meta},
    server::run_server,
    utils::zilliqa::Zilliqa,
};
use log::{error, LevelFilter};
use simple_logger::SimpleLogger;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio;

#[tokio::main]
async fn main() {
    SimpleLogger::new()
        .with_colors(true)
        .with_level(LevelFilter::Info)
        .init()
        .unwrap();

    let meta = Arc::new(Mutex::new(Meta::new()));
    let rates = Arc::new(Mutex::new(Currencies::new()));
    let dex = Arc::new(Mutex::new(Dex::new()));

    let meta_worker_ref = Arc::clone(&meta);
    let rates_worker_ref = Arc::clone(&rates);
    let dex_worker_ref = Arc::clone(&dex);

    tokio::task::spawn(async move {
        let zil = Zilliqa::new();

        loop {
            let mut rates = match rates_worker_ref.lock() {
                Ok(rates) => rates.update().await,
                Err(e) => {
                    error!("RATES: {:?}", e);
                    continue;
                }
            };
            let mut dex = match dex_worker_ref.lock() {
                Ok(dex) => {
                    dex.update(&zil).await;

                    dex
                }
                Err(e) => {
                    error!("DEX: {:?}", e);
                    continue;
                }
            };
            let mut meta = match meta_worker_ref.lock() {
                Ok(meta) => {
                    meta.update(&zil).await;
                    meta.listed_tokens_update(&dex);

                    meta
                }
                Err(e) => {
                    error!("META: {:?}", e);
                    continue;
                }
            };

            drop(meta);
            drop(rates);
            drop(dex);

            tokio::time::sleep(Duration::from_secs(300)).await;
        }
    });

    run_server(meta, dex, rates).await.unwrap();
}
