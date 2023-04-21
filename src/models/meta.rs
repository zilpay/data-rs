use std::u8;

use crate::config::meta::{CRYPTO_META_URL, TOKENS_EXCEPTIONS};
use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Map, Value};

#[derive(Debug, Serialize)]
pub struct Token {
    pub bech32: String,
    pub score: u8,
}

#[derive(Debug)]
pub struct Meta {
    pub list: Vec<Token>,
}

impl Meta {
    pub fn new() -> Self {
        let list = Vec::new();
        Meta { list }
    }

    pub async fn update(&mut self) {
        match self.fetch().await {
            Ok(list) => {
                self.list = list;
                ()
            }
            Err(_) => (),
        };
    }

    pub fn serializatio(&self) -> String {
        serde_json::to_string(&self.list).unwrap()
    }

    async fn fetch(&self) -> Result<Vec<Token>, reqwest::Error> {
        let client = Client::new();
        let response = client.get(CRYPTO_META_URL).send().await?;
        let chain = "zilliqa.";
        let body: Map<String, Value> = response.json().await?;
        let body: Vec<Token> = body
            .into_iter()
            .filter(|(key, _)| key.contains(chain))
            .map(|(key, value)| {
                let bech32 = key.replace(chain, "");
                let found_exceptions = TOKENS_EXCEPTIONS.iter().find(|&addr| addr[0] == bech32);
                let score: u8 = match value.get("gen") {
                    Some(gen) => {
                        let score = gen.get("score").unwrap_or(&json!(0)).as_u64().unwrap_or(0);
                        let score: u8 = score.try_into().unwrap_or(0);

                        score
                    }
                    None => 0,
                };

                match found_exceptions {
                    Some(found) => Token {
                        score,
                        bech32: String::from(found[1]),
                    },
                    None => Token { bech32, score },
                }
            })
            .collect();

        Ok(body)
    }
}
