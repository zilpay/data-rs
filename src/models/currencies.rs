use crate::config::currencies::CURRENCIES;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Serialize)]
pub struct Currencies {
    pub data: Map<String, Value>,
}

#[derive(Deserialize, Debug)]
struct APIResponse {
    zilliqa: Map<String, Value>,
}

impl Currencies {
    pub fn new() -> Self {
        let mut data = serde_json::Map::new();

        for currency in CURRENCIES {
            data.insert(currency.to_lowercase().to_owned(), Value::from(0.0));
        }

        Currencies { data }
    }

    pub fn serializatio(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }

    pub async fn update(&mut self) {
        self.data = self.coingecko().await.unwrap();
    }

    async fn coingecko(&self) -> Result<Map<String, Value>, reqwest::Error> {
        let client = Client::new();
        let params = format!("?ids=zilliqa&vs_currencies={:?}", CURRENCIES.join(","));
        let url = format!("https://api.coingecko.com/api/v3/simple/price{}", params);
        let response = client.get(url).send().await?;

        let body: APIResponse = response.json().await?;

        Ok(body.zilliqa)
    }
}
