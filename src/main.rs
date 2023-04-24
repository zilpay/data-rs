// use data_rs::models::currencies::Currencies;
use data_rs::{models::meta::Meta, utils::zilliqa::Zilliqa};
use tokio;

#[tokio::main]
async fn main() {
    let zil = Zilliqa::new();
    let mut meta = Meta::new();

    meta.update(&zil).await.unwrap();
}
