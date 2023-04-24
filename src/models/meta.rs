use std::u8;

use crate::{
    config::{
        meta::{CRYPTO_META_URL, MIN_SCORE, TOKENS_EXCEPTIONS},
        zilliqa::RPC_METHODS,
    },
    utils::{
        crypto::from_bech32_address,
        zilliqa::{JsonBodyReq, JsonBodyRes, Zilliqa},
    },
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::io::{Error, ErrorKind};

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
    pub bech32: String,
    pub base16: String,
    pub score: u8,
    pub name: String,
    pub symbol: String,
    pub token_type: u8, // 1 = ZRC1, 2 = ZRC2
    pub decimals: u8,
}

#[derive(Debug, Deserialize, Serialize)]
struct ContractInit {
    #[serde(flatten)]
    pub value: serde_json::Value,
    pub vname: String,
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
        let bodies: Vec<JsonBodyReq> = tokens
            .iter()
            .map(|(_, _, base16)| {
                let params = vec![base16.to_string()];

                zilliqa.build_body(RPC_METHODS.get_smart_contract_init, params)
            })
            .collect();
        let res: Vec<JsonBodyRes<Vec<ContractInit>>> = zilliqa.fetch(bodies).await?;
        let new_tokens: Vec<Token> = res
            .iter()
            .filter_map(|r| {
                let token_type = 1; // TODO: track only ZRC2 tokens.
                let params = match &r.result {
                    Some(result) => result,
                    None => return None,
                };
                let (name, symbol, base16, decimals) = match self.parse_init(params) {
                    Ok(tuple) => tuple,
                    Err(_) => return None,
                };
                dbg!(&name, &symbol, &base16, &decimals);
                let (bech32, score, _) = match tokens
                    .iter()
                    .find(|(_, _, base16)| base16.replace("0x", "") == *base16)
                {
                    Some(f) => f.clone(),
                    None => return None,
                };

                Some(Token {
                    bech32,
                    base16,
                    decimals,
                    name,
                    symbol,
                    token_type,
                    score,
                })
            })
            .collect();

        dbg!(&new_tokens);

        Ok(())
    }

    pub fn serializatio(&self) -> String {
        serde_json::to_string(&self.list).unwrap()
    }

    async fn fetch(&self) -> Result<Vec<(String, u8, String)>, reqwest::Error> {
        let client = Client::new();
        let response = client.get(CRYPTO_META_URL).send().await?;
        let chain = "zilliqa.";
        let body: Map<String, Value> = response.json().await?;
        let body: Vec<(String, u8, String)> = body
            .into_iter()
            .filter(|(key, _)| key.contains(chain))
            .filter_map(|(key, value)| {
                let bech32 = key.replace(chain, "");
                let base16 = match from_bech32_address(&bech32) {
                    Some(addr) => hex::encode(addr),
                    None => return None,
                };

                // Skip Already has tokens
                match self.list.iter().find(|t| t.bech32 == bech32) {
                    Some(_) => return None,
                    None => (),
                }

                let found_exceptions = TOKENS_EXCEPTIONS.iter().find(|&addr| addr[0] == bech32);
                let score: u8 = match value.get("gen") {
                    Some(gen) => {
                        let score = gen.get("score").unwrap_or(&json!(0)).as_u64().unwrap_or(0);
                        let score: u8 = score.try_into().unwrap_or(0);

                        score
                    }
                    None => 0,
                };

                if score < MIN_SCORE {
                    return None;
                }

                match found_exceptions {
                    Some(found) => Some((
                        String::from(found[1]),
                        score,
                        hex::encode(from_bech32_address(&found[1]).unwrap()),
                    )),
                    None => Some((bech32, score, base16)),
                }
            })
            .collect();

        Ok(body)
    }

    fn parse_init(
        &self,
        params: &Vec<ContractInit>,
    ) -> Result<(String, String, String, u8), Error> {
        let key = "value";
        let name = match params.iter().find(|item| item.vname == "name") {
            Some(n) => n
                .value
                .get(key)
                .unwrap_or(&json!(""))
                .as_str()
                .unwrap_or("")
                .to_string(),
            None => {
                return Err(Error::new(ErrorKind::Other, "vname (name) is required"));
            }
        };
        let symbol = match params.iter().find(|item| item.vname == "symbol") {
            Some(n) => n
                .value
                .get(key)
                .unwrap_or(&json!(""))
                .as_str()
                .unwrap_or("")
                .to_string(),
            None => {
                return Err(Error::new(ErrorKind::Other, "vname (symbol) is required"));
            }
        };
        let base16 = match params.iter().find(|item| item.vname == "_this_address") {
            Some(n) => n
                .value
                .get(key)
                .unwrap_or(&json!(""))
                .as_str()
                .unwrap_or("")
                .to_lowercase(),
            None => {
                return Err(Error::new(
                    ErrorKind::Other,
                    "vname (_this_address) is required",
                ));
            }
        };
        let decimals = match params.iter().find(|item| item.vname == "decimals") {
            Some(n) => n.value.get(key).unwrap_or(&json!(0)).as_u64().unwrap_or(0) as u8,
            None => {
                return Err(Error::new(ErrorKind::Other, "vname (decimals) is required"));
            }
        };

        Ok((name, symbol, base16, decimals))
    }
}
