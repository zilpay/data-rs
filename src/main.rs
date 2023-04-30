use async_mutex::Mutex;
use data_rs::{
    models::{currencies::Currencies, dex::Dex, meta::Meta},
    server::run_server,
    utils::zilliqa::Zilliqa,
};
use log::LevelFilter;
use simple_logger::SimpleLogger;
use std::sync::Arc;
use std::time::Duration;
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

    let meta_worker_ref = meta.clone();
    let rates_worker_ref = Arc::clone(&rates);
    let dex_worker_ref = Arc::clone(&dex);

    tokio::task::spawn(async move {
        let zil = Zilliqa::new();

        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;

            let mut meta_guard = meta_worker_ref.lock().await;
            let mut rates = rates_worker_ref.lock().await;
            let mut dex = dex_worker_ref.lock().await;

            rates.update().await.unwrap();
            meta_guard.update(&zil).await.unwrap();
            dex.update(&zil).await.unwrap();
        }
    });

    run_server(meta, dex, rates).await.unwrap();
}
