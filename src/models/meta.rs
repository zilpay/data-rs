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
use sled::Db;
use std::{
    collections::HashSet,
    io::{Error, ErrorKind},
};

use super::dex::Dex;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Token {
    pub bech32: String,
    pub base16: String,
    pub scope: u8,
    pub name: String,
    pub symbol: String,
    pub token_type: u8,
    pub decimals: u8,
    pub listed: bool,
    pub status: u8,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ContractInit {
    #[serde(flatten)]
    pub value: serde_json::Value,
    pub vname: String,

    #[serde(rename = "type")]
    pub field_type: String,
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
        let db =
            sled::open(format!("{}/{}", db_path, META_KEY)).expect("Cannot meta open database.");

        let list = db
            .get(META_KEY)
            .map(|mb_cache| {
                let cache = mb_cache.unwrap_or_default();
                std::str::from_utf8(&cache)
                    .map(|mb_json| serde_json::from_str(mb_json).unwrap_or_default())
                    .unwrap_or_default()
            })
            .unwrap_or_else(|_| {
                error!("{app_name}: fail to load cache!");
                Vec::new()
            });

        info!("{app_name}: loaded from cache {}", list.len());
        Meta { list, db, app_name }
    }

    pub fn update(
        &mut self,
        tokens: Vec<(String, u8, String)>,
        res: Vec<JsonBodyRes<Vec<ContractInit>>>,
    ) -> Result<(), std::io::Error> {
        let existing_base16s: HashSet<String> =
            self.list.iter().map(|t| t.base16.to_lowercase()).collect();

        let new_tokens: Vec<Token> = res
            .iter()
            .filter_map(|r| {
                let params = r.result.as_ref()?;
                let (name, symbol, base16, decimals) = Meta::parse_init(params).ok()?;

                if existing_base16s.contains(&base16.to_lowercase()) {
                    return None;
                }

                let (bech32, scope, _) = tokens
                    .iter()
                    .find(|(_, _, b16)| {
                        b16.to_lowercase().replace("0x", "")
                            == base16.replace("0x", "").to_lowercase()
                    })?
                    .clone();

                Some(Token {
                    bech32,
                    status: 1,
                    base16,
                    decimals,
                    name,
                    symbol,
                    token_type: 1,
                    scope,
                    listed: false,
                })
            })
            .collect();

        info!("{}: added new tokens {}", self.app_name, new_tokens.len());

        self.list.extend(new_tokens);
        self.write_db()?;

        Ok(())
    }

    pub fn write_db(&mut self) -> Result<(), Error> {
        self.list.sort_by(|a, b| b.scope.cmp(&a.scope));
        self.db.insert(META_KEY, self.serialization().as_bytes())?;
        Ok(())
    }

    pub async fn sort_zilliqa_tokens(
        tokens: &Vec<(String, u8, String)>,
        zilliqa: &Zilliqa,
    ) -> Result<Vec<JsonBodyRes<Vec<ContractInit>>>, Error> {
        let bodies: Vec<JsonBodyReq> = tokens
            .iter()
            .map(|(_, _, base16)| {
                zilliqa.build_body(RPC_METHODS.get_smart_contract_init, json!([base16]))
            })
            .collect();

        let results = zilliqa.fetch::<Vec<ContractInit>>(bodies).await?;
        Ok(results)
    }

    pub async fn get_meta_tokens() -> Result<Vec<(String, u8, String)>, Error> {
        Meta::fetch().await.map_err(|e| {
            error!("Github is down!, error: {:?}", e);
            Error::new(ErrorKind::Other, "Github is down")
        })
    }

    pub fn listed_tokens_update(&mut self, dex: &Dex) {
        for token in &mut self.list {
            token.listed = dex.pools.contains_key(&token.base16);
        }
    }

    pub fn serialization(&self) -> String {
        serde_json::to_string(&self.list).unwrap_or_default()
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
                let base16 = from_bech32_address(&bech32).map(hex::encode)?;

                let found_exceptions = TOKENS_EXCEPTIONS.iter().find(|&addr| addr[0] == bech32);
                let score: u8 = value
                    .get("gen")
                    .and_then(|gen| gen.get("score"))
                    .and_then(|s| s.as_u64())
                    .map(|s| s as u8)
                    .unwrap_or(0);

                if score < MIN_SCORE {
                    return None;
                }

                match found_exceptions {
                    Some(found) => from_bech32_address(&found[1])
                        .map(|addr| (String::from(found[1]), score, hex::encode(addr))),
                    None => Some((bech32, score, base16)),
                }
            })
            .collect();

        Ok(body)
    }

    fn parse_init(params: &Vec<ContractInit>) -> Result<(String, String, String, u8), Error> {
        let get_string_value = |vname: &str| -> Result<String, Error> {
            params
                .iter()
                .find(|item| item.vname == vname)
                .and_then(|n| n.value.get("value"))
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| {
                    Error::new(ErrorKind::Other, format!("vname ({}) is required", vname))
                })
        };

        let name = get_string_value("name")?;
        let symbol = get_string_value("symbol")?;
        let base16 = get_string_value("_this_address")?.to_lowercase();

        let decimals = params
            .iter()
            .find(|item| item.vname == "decimals")
            .and_then(|n| n.value.get("value"))
            .and_then(|v| v.as_str())
            .map(|s| s.parse::<u8>().unwrap_or(0))
            .ok_or_else(|| Error::new(ErrorKind::Other, "vname (decimals) is required"))?;

        Ok((name, symbol, base16, decimals))
    }
}
