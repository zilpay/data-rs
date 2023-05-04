use std::u8;

use crate::{
    config::{
        meta::{CRYPTO_META_URL, META_KEY, MIN_SCORE, TOKENS_EXCEPTIONS},
        zilliqa::RPC_METHODS,
    },
    utils::{
        crypto::from_bech32_address,
        zilliqa::{JsonBodyReq, JsonBodyRes, Zilliqa},
    },
};
use log::{error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sled::{Db, IVec};
use std::io::{Error, ErrorKind};

use super::dex::Dex;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Token {
    pub bech32: String,
    pub base16: String,
    pub score: u8,
    pub name: String,
    pub symbol: String,
    pub token_type: u8, // 1 = ZRC1, 2 = ZRC2
    pub decimals: u8,
    pub listed: bool,
    pub status: u8, // 0 - blocked, 1 - enabled
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContractInit {
    #[serde(flatten)]
    pub value: serde_json::Value,
    pub vname: String,
}

#[derive(Debug)]
pub struct Meta {
    pub list: Vec<Token>,
    db: Db,
    app_name: &'static str,
}

impl Meta {
    pub fn new(db_path: &str) -> Self {
        let app_name = "META";
        let db = sled::open(db_path).expect("Cannot meta open database.");
        let list = match db.get(META_KEY) {
            Ok(mb_cache) => {
                let cache = mb_cache.unwrap_or(IVec::default());
                let mb_json = std::str::from_utf8(&cache).unwrap();
                let list = serde_json::from_str(mb_json).unwrap_or(Vec::new());

                info!("{app_name}: loaded from cache {}", list.len());

                list
            }
            Err(_) => {
                error!("{app_name}: fail to load cache!");

                Vec::new()
            }
        };

        Meta { list, db, app_name }
    }

    pub fn update(
        &mut self,
        tokens: Vec<(String, u8, String)>,
        res: Vec<JsonBodyRes<Vec<ContractInit>>>,
    ) -> Result<(), std::io::Error> {
        let new_tokens: Vec<Token> = res
            .iter()
            .filter_map(|r| {
                let listed = false;
                let token_type = 1; // TODO: track only ZRC2 tokens.
                let status = 1;
                let params = match &r.result {
                    Some(result) => result,
                    None => return None,
                };
                let (name, symbol, base16, decimals) = match Meta::parse_init(params) {
                    Ok(tuple) => tuple,
                    Err(_) => return None,
                };

                // Skip Already has tokens
                match self.list.iter().find(|t| t.base16 == base16) {
                    Some(_) => return None,
                    None => (),
                }

                let (bech32, score, _) = match tokens
                    .iter()
                    .find(|(_, _, base16)| base16.replace("0x", "") == *base16)
                {
                    Some(f) => f.clone(),
                    None => return None,
                };

                Some(Token {
                    bech32,
                    status,
                    base16,
                    decimals,
                    name,
                    symbol,
                    token_type,
                    score,
                    listed,
                })
            })
            .collect();

        info!(
            "{:?}: added new tokens {:?}",
            self.app_name,
            new_tokens.len()
        );

        self.list.extend(new_tokens);
        self.write_db()?;

        Ok(())
    }

    pub fn write_db(&self) -> Result<(), Error> {
        self.db.insert(META_KEY, self.serializatio().as_bytes())?;

        Ok(())
    }

    pub async fn sort_zilliqa_tokens(
        tokens: &Vec<(String, u8, String)>,
        zilliqa: &Zilliqa,
    ) -> Result<Vec<JsonBodyRes<Vec<ContractInit>>>, Error> {
        let bodies: Vec<JsonBodyReq> = tokens
            .iter()
            .map(|(_, _, base16)| {
                let params = json!([base16]);

                zilliqa.build_body(RPC_METHODS.get_smart_contract_init, params)
            })
            .collect();
        let res: Vec<JsonBodyRes<Vec<ContractInit>>> = zilliqa.fetch(bodies).await?;

        Ok(res)
    }

    pub async fn get_meta_tokens() -> Result<Vec<(String, u8, String)>, Error> {
        match Meta::fetch().await {
            Ok(tokens) => return Ok(tokens),
            Err(e) => {
                let custom_error = Error::new(ErrorKind::Other, "Github is down");

                error!("Github is down!, error: {:?}", e);

                return Err(custom_error);
            }
        };
    }

    pub fn listed_tokens_update(&mut self, dex: &Dex) {
        for token in self.list.iter_mut() {
            token.listed = dex.pools.contains_key(&token.base16);
        }
    }

    pub fn serializatio(&self) -> String {
        serde_json::to_string(&self.list).unwrap()
    }

    async fn fetch() -> Result<Vec<(String, u8, String)>, reqwest::Error> {
        let client = Client::new();
        let response = client.get(CRYPTO_META_URL).send().await?;
        let chain = "zilliqa.";
        let body: Map<String, Value> = response.json().await?;
        let body: Vec<(String, u8, String)> = body
            .into_iter()
            .filter_map(|(key, value)| {
                if !key.contains(chain) {
                    return None;
                }

                let bech32 = key.replace(chain, "");
                let base16 = match from_bech32_address(&bech32) {
                    Some(addr) => hex::encode(addr),
                    None => return None,
                };

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

    fn parse_init(params: &Vec<ContractInit>) -> Result<(String, String, String, u8), Error> {
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
            Some(n) => {
                let str_value = n
                    .value
                    .get(key)
                    .unwrap_or(&json!("0"))
                    .as_str()
                    .unwrap_or("0")
                    .to_string();
                let decimals: u8 = str_value.parse().unwrap_or(0);

                decimals
            }
            None => {
                return Err(Error::new(ErrorKind::Other, "vname (decimals) is required"));
            }
        };

        Ok((name, symbol, base16, decimals))
    }
}
