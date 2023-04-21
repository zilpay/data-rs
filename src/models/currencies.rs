use crate::config::currencies::CURRENCIES;
use reqwest::Client;
use serde_json::{Map, Value};
use tokio;

pub struct Currencies {
    data: Map<String, Value>,
}

impl Currencies {
    pub fn new() -> Self {
        let mut data = serde_json::Map::new();

        for currency in CURRENCIES {
            data.insert(currency.to_lowercase().to_owned(), Value::from(0.0));
        }

        Currencies { data }
    }

    pub fn update(&self) {}

    async fn coingecko(&self) -> Result<(), reqwest::Error> {
        let client = Client::new();
        let params = format!("?ids=zilliqa&vs_currencies={:?}", CURRENCIES.join(","));
        let url = format!("https://api.coingecko.com/api/v3/simple/price{}", params);
        let response = client.get(url).send().await?;

        if response.status().is_success() {
            let body = response.json().await?;
            println!("Response body: {:?}", body);
        } else {
            println!("Error: {}", response.status());
        }

        Ok(())
    }
}
