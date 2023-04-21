use data_rs::models::currencies::Currencies;
use tokio;

#[tokio::main]
async fn main() {
    let mut rates = Currencies::new();

    rates.update().await;

    dbg!(rates.serializatio());
}
