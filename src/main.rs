// use data_rs::models::currencies::Currencies;
use data_rs::{models::meta::Meta, utils::zilliqa::Zilliqa};
use std::env;
use tokio;

#[tokio::main]
async fn main() {
    env::set_var("RUST_LOG", "info,warn,error");

    let zil = Zilliqa::new();
    let mut meta = Meta::new();

    meta.update(&zil).await.unwrap();
    // dbg!(meta.serializatio());
}
