// use data_rs::models::currencies::Currencies;
use data_rs::models::meta::Meta;
use tokio;

#[tokio::main]
async fn main() {
    // let rates = Currencies::new();
    let meta = Meta::new();

    meta.update().await;

    // rates.update().await;
}
