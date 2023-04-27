// use data_rs::models::currencies::Currencies;
use data_rs::models::dex::Dex;
use data_rs::{models::meta::Meta, utils::zilliqa::Zilliqa};
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
    let zil = Zilliqa::new();
    let mut dex = Dex::new();
    let mut tokens = Meta::new();

    tokens.update(&zil).await;
    dex.update(&zil).await;

    tokens.listed_tokens_update(&dex);

    dbg!(&tokens.list);
}
