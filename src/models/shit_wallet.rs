use std::collections::HashMap;
use std::io::{Error, ErrorKind};

use crate::config::blockchain::{BLOCKCHAIN_KEY, BLOCK_NUMBER_KEY, START_INDEX_BLOCK};
use crate::config::zilliqa::RPC_METHODS;
use crate::config::zilliqa::ZERO_ADDR;
use crate::utils::crypto::get_address_from_public_key;
use crate::utils::zilliqa::{JsonBodyReq, JsonBodyRes, Zilliqa};
use log::info;
use serde_json::{json, Map, Value};
use sled::{Db, IVec};

#[derive(Debug)]
pub struct ShitWallet {
    pub wallets: HashMap<String, Vec<String>>,
    pub current_block: u64,
    pub db: Db,
    app_name: &'static str,
}

impl ShitWallet {
    pub fn new(db_path: &str) -> Self {
        let app_name = "BLOCKCHAIN";
        let db = sled::open(format!("{}/{}", db_path, BLOCKCHAIN_KEY))
            .expect("Cannot open currencies database.");
        let wallets: HashMap<String, Vec<String>> = match db.get(BLOCKCHAIN_KEY) {
            Ok(mb_cache) => {
                let cache = mb_cache.unwrap_or(IVec::default());
                let mb_json = std::str::from_utf8(&cache).unwrap_or("{}");

                serde_json::from_str(mb_json).unwrap_or(HashMap::new())
            }
            Err(_) => HashMap::new(),
        };
        let current_block = match db.get(BLOCK_NUMBER_KEY) {
            Ok(mb_block) => {
                let cache = mb_block.unwrap_or(IVec::default());
                let value = u64::from_be_bytes(
                    cache
                        .as_ref()
                        .try_into()
                        .unwrap_or(START_INDEX_BLOCK.to_be_bytes()),
                );

                value
            }
            Err(_) => START_INDEX_BLOCK,
        };

        ShitWallet {
            current_block,
            wallets,
            db,
            app_name,
        }
    }

    pub fn wallet_serializatio(&self) -> String {
        serde_json::to_string(&self.wallets).unwrap()
    }

    pub fn update_block(&mut self, block_number: u64) -> Result<(), Error> {
        self.current_block = block_number;
        self.db
            .insert(BLOCK_NUMBER_KEY, &block_number.to_be_bytes())?;

        info!(
            "{:?}: block number {:?} updated!",
            self.app_name, self.current_block
        );

        Ok(())
    }

    pub fn update_wallets(&mut self, wallets: Vec<(String, String)>) -> Result<(), Error> {
        for w in wallets {
            match self.wallets.get_mut(&w.1) {
                Some(value) => {
                    value.push(w.0);
                }
                None => {
                    self.wallets.insert(w.1, vec![w.0]);
                }
            };
        }

        self.db
            .insert(BLOCKCHAIN_KEY, self.wallet_serializatio().as_bytes())?;

        info!("{:?}: rates updated!", self.app_name);

        Ok(())
    }

    pub async fn get_block_body(
        zilliqa: &Zilliqa,
        block_numbers: &Vec<u64>,
    ) -> Result<Vec<(String, String)>, Error> {
        let mut mb_wallets: Vec<(String, String)> = Vec::new();
        let bodies: Vec<JsonBodyReq> = block_numbers
            .iter()
            .map(|block_number| {
                let params = json!([block_number.to_string()]);
                zilliqa.build_body(RPC_METHODS.get_txn_bodies_for_tx_block, params)
            })
            .collect();
        let res: Vec<JsonBodyRes<Vec<Map<String, Value>>>> = zilliqa.fetch(bodies).await.unwrap();
        let bodies: Vec<Vec<Map<String, Value>>> =
            res.into_iter().filter_map(|b| b.result).collect();

        for txns in bodies {
            for tx in txns {
                if let Value::String(to_addr) = tx.get("toAddr").unwrap_or(&json!("")) {
                    if to_addr != ZERO_ADDR {
                        // Only deploy contract txns.
                        continue;
                    }

                    let receipt = tx.get("receipt").unwrap_or(&json!("")).clone();
                    let success = receipt
                        .get("success")
                        .clone()
                        .unwrap_or(&json!(false))
                        .as_bool()
                        .unwrap_or(false);

                    if !success {
                        continue;
                    }

                    let init = match tx.get("data") {
                        Some(d) => d,
                        None => continue,
                    };
                    let init_admin_pubkey = match ShitWallet::get_pub_from_init(init) {
                        Ok(key) => key,
                        Err(_) => continue,
                    };
                    let addr = match get_address_from_public_key(&init_admin_pubkey) {
                        Ok(a) => a,
                        Err(_) => continue,
                    };
                    let hash = match tx.get("ID") {
                        Some(h) => {
                            if let Value::String(str) = h {
                                str
                            } else {
                                continue;
                            }
                        }
                        None => continue,
                    };

                    mb_wallets.push((hash.to_owned(), addr.to_owned()));
                } else {
                    continue;
                };
            }
        }

        Ok(mb_wallets)
    }

    pub async fn fetch_wallets(
        zil: &Zilliqa,
        wallets: Vec<(String, String)>,
    ) -> Result<Vec<(String, String)>, Error> {
        let req_bodies: Vec<JsonBodyReq> = wallets
            .iter()
            .map(|(hash, _)| {
                let params = json!([hash]);

                zil.build_body(RPC_METHODS.get_contract_address_from_transaction_id, params)
            })
            .collect();
        let res: Vec<JsonBodyRes<String>> = zil.fetch(req_bodies).await?;
        let contracts: Vec<String> = res.iter().filter_map(|r| r.result.clone()).collect();
        let mut shit_wallets: Vec<(String, String)> = Vec::new();

        for (index, wallet) in wallets.iter().enumerate() {
            let contract = match contracts.get(index) {
                Some(c) => c,
                None => continue,
            };

            shit_wallets.push((contract.to_owned(), wallet.1.to_owned()));
        }

        Ok(shit_wallets)
    }

    pub async fn get_later_block(zilliqa: &Zilliqa) -> Result<u64, Error> {
        let custom_error = Error::new(ErrorKind::Other, "Fail to fetch or parse blockchain info");
        let params = json!([]);
        let bodies: Vec<JsonBodyReq> =
            vec![zilliqa.build_body(RPC_METHODS.get_blockchain_info, params)];
        let res: Vec<JsonBodyRes<Map<String, Value>>> = zilliqa.fetch(bodies).await?;
        let last_block = match res.first() {
            Some(info) => {
                let tx_block_number = info.result.clone().unwrap();
                let value = tx_block_number
                    .get("NumTxBlocks")
                    .unwrap_or(&json!(""))
                    .clone();

                if let Value::String(s) = value {
                    s
                } else {
                    String::new()
                }
            }
            None => return Err(custom_error),
        };

        match last_block.parse::<u64>() {
            Ok(n) => Ok(n),
            Err(_) => Err(custom_error),
        }
    }

    fn get_pub_from_init(raw: &Value) -> Result<String, Error> {
        let broken_init = Error::new(ErrorKind::Other, "Fail to parse init with pubKey");

        if let Value::String(json) = raw {
            let parsed_json: Value = serde_json::from_str(json)?;

            if let Value::Array(list) = parsed_json {
                for init in list {
                    if init.get("vname").unwrap_or(&json!("")) == &json!("init_admin_pubkey") {
                        if let Value::String(pub_key) = init.get("value").unwrap_or(&json!("")) {
                            if pub_key.len() < 68 {
                                return Err(broken_init);
                            }

                            return Ok(pub_key.to_owned());
                        } else {
                            return Err(broken_init);
                        }
                    }
                }
            }
        } else {
            let custom_error = Error::new(ErrorKind::Other, "Fail to parse init");

            return Err(custom_error);
        }

        Err(broken_init)
    }
}
