use data_rs::models::currencies::Currencies;
use data_rs::models::dex::Dex;
use data_rs::{models::meta::Meta, utils::zilliqa::Zilliqa};
use tokio;

#[tokio::main]
async fn main() {
    let zil = Zilliqa::new();
    let mut dex = Dex::new();

    dex.update(&zil).await;
}
