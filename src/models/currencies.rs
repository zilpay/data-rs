use crate::config::currencies::{CURRENCIES, CURRENCIES_DATABASE, CURRENCIES_KEY};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{Map, Value};
use sled::{Db, IVec};

#[derive(Debug)]
pub struct Currencies {
    pub data: Map<String, Value>,
    db: Db,
}

#[derive(Deserialize, Debug)]
struct APIResponse {
    zilliqa: Map<String, Value>,
}

impl Currencies {
    pub fn new() -> Self {
        let db = sled::open(CURRENCIES_DATABASE).expect("Cannot open currencies database.");
        let data = match db.get(CURRENCIES_KEY) {
            Ok(mb_cache) => {
                let cache = mb_cache.unwrap_or(IVec::default());
                let mb_json = std::str::from_utf8(&cache).unwrap_or("{}");

                serde_json::from_str(mb_json).unwrap_or(Map::new())
            }
            Err(_) => {
                let mut data = Map::new();

                for currency in CURRENCIES {
                    data.insert(currency.to_lowercase().to_owned(), Value::from(0.0));
                }

                data
            }
        };

        Currencies { data, db }
    }

    pub fn serializatio(&self) -> String {
        serde_json::to_string(&self.data).unwrap()
    }

    pub async fn update(&mut self) {
        self.data = self.coingecko().await.unwrap();
        self.db
            .insert(CURRENCIES_KEY, self.serializatio().as_bytes())
            .expect("cannot insert");
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
