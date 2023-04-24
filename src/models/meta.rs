use std::u8;

use crate::{
    config::{
        meta::{CRYPTO_META_URL, TOKENS_EXCEPTIONS},
        zilliqa::RPC_METHODS,
    },
    utils::{
        crypto::from_bech32_address,
        zilliqa::{JsonBodyReq, JsonBodyRes, Zilliqa},
    },
};
use reqwest::Client;
use serde::Serialize;
use serde_json::{json, Map, Value};
use std::io::{Error, ErrorKind};

#[derive(Debug, Serialize)]
pub struct Token {
    pub bech32: String,
    pub base16: String,
    pub score: u8,
    pub name: String,
    pub symbol: String,
    pub token_type: u8, // 1 = ZRC1, 2 = ZRC2
    pub decimals: u8,
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

    pub async fn update(&mut self, zilliqa: &Zilliqa) -> Result<(), std::io::Error> {
        let tokens = match self.fetch().await {
            Ok(tokens) => tokens,
            Err(_) => {
                let custom_error = Error::new(ErrorKind::Other, "Github is down");

                return Err(custom_error);
            }
        };
        // TODO: add filter new and which already stored
        let bodies: Vec<JsonBodyReq> = tokens
            .iter()
            .filter_map(|(bech32, _)| {
                let base16_buff = match from_bech32_address(bech32) {
                    Some(buff) => buff,
                    None => return None,
                };
                let params = vec![hex::encode(base16_buff)];

                Some(zilliqa.build_body(RPC_METHODS.get_smart_contract_init, params))
            })
            .collect();
        let res: Vec<JsonBodyRes<Vec<Map<String, Value>>>> = zilliqa.fetch(bodies).await.unwrap();

        dbg!(res);

        Ok(())
    }

    pub fn serializatio(&self) -> String {
        serde_json::to_string(&self.list).unwrap()
    }

    async fn fetch(&self) -> Result<Vec<(String, u8)>, reqwest::Error> {
        let client = Client::new();
        let response = client.get(CRYPTO_META_URL).send().await?;
        let chain = "zilliqa.";
        let body: Map<String, Value> = response.json().await?;
        let body: Vec<(String, u8)> = body
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
                    Some(found) => (String::from(found[1]), score),
                    None => (bech32, score),
                }
            })
            .collect();

        Ok(body)
    }
}
