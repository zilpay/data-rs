use std::io::{Error, ErrorKind};

use crate::config::currencies::{CURRENCIES, CURRENCIES_KEY};
use log::{error, info};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{Map, Value};
use sled::{Db, IVec};

#[derive(Debug)]
pub struct Currencies {
    pub data: Map<String, Value>,
    db: Db,
    app_name: &'static str,
}

#[derive(Deserialize, Debug)]
struct APIResponse {
    zilliqa: Map<String, Value>,
}

impl Currencies {
    pub fn new(db_path: &str) -> Self {
        let app_name = "RATES";
        let db = sled::open(db_path).expect("Cannot open currencies database.");
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

        Currencies { data, db, app_name }
    }

    pub fn serializatio(&self) -> String {
        serde_json::to_string(&self.data).unwrap()
    }

    pub fn update(&mut self, rates: Map<String, Value>) -> Result<(), Error> {
        self.data = rates;
        self.db
            .insert(CURRENCIES_KEY, self.serializatio().as_bytes())?;

        info!("{:?}: rates updated!", self.app_name);

        Ok(())
    }

    pub async fn fetch_rates() -> Result<Map<String, Value>, Error> {
        let data = match Currencies::coingecko().await {
            Ok(data) => data,
            Err(e) => {
                let custom_error = Error::new(ErrorKind::Other, "coingecko is down");

                error!("coingecko: cannot load rates, error: {:?}", e);

                return Err(custom_error);
            }
        };

        Ok(data)
    }

    async fn coingecko() -> Result<Map<String, Value>, reqwest::Error> {
        let client = Client::new();
        let url = format!(
            "https://api.coingecko.com/api/v3/simple/price?ids=zilliqa&vs_currencies={}",
            CURRENCIES.join(",")
        )
        .to_lowercase();
        let response = client.get(url).send().await?;

        let body: APIResponse = response.json().await?;

        Ok(body.zilliqa)
    }
}
