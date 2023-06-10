use std::io::{Error, ErrorKind};

use crate::config::blockchain::{BLOCKCHAIN_KEY, BLOCK_NUMBER_KEY, START_INDEX_BLOCK};
use crate::config::zilliqa::RPC_METHODS;
use crate::utils::zilliqa::{JsonBodyReq, JsonBodyRes, Zilliqa};
use log::{error, info};
use serde::Deserialize;
use serde_json::{json, to_string, Map, Value};
use sled::{Db, IVec};

#[derive(Debug)]
pub struct ShitWallet {
    pub wallets: Map<String, Value>,
    pub current_block: u64,
    pub db: Db,
    app_name: &'static str,
}

impl ShitWallet {
    pub fn new(db_path: &str) -> Self {
        let app_name = "BLOCKCHAIN";
        let db = sled::open(format!("{}/{}", db_path, BLOCKCHAIN_KEY))
            .expect("Cannot open currencies database.");
        let wallets = match db.get(BLOCKCHAIN_KEY) {
            Ok(mb_cache) => {
                let cache = mb_cache.unwrap_or(IVec::default());
                let mb_json = std::str::from_utf8(&cache).unwrap_or("{}");

                serde_json::from_str(mb_json).unwrap_or(Map::new())
            }
            Err(_) => Map::new(),
        };
        let current_block = match db.get(BLOCK_NUMBER_KEY) {
            Ok(mb_block) => {
                let cache = mb_block.unwrap_or(IVec::default());
                let value = u64::from_be_bytes(cache.as_ref().try_into().unwrap());

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

    pub fn serializatio(&self) -> String {
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

    pub fn update_wallets(&mut self, wallets: Map<String, Value>) -> Result<(), Error> {
        self.wallets = wallets;
        self.db
            .insert(BLOCKCHAIN_KEY, self.serializatio().as_bytes())?;

        info!("{:?}: rates updated!", self.app_name);

        Ok(())
    }

    pub async fn get_block_body(&self, zilliqa: &Zilliqa, block_number: u16) {
        let params = json!([block_number.to_string()]);
        let bodies: Vec<JsonBodyReq> =
            vec![zilliqa.build_body(RPC_METHODS.get_txn_bodies_for_tx_block, params)];
        let res: Vec<JsonBodyRes<Map<String, Value>>> = zilliqa.fetch(bodies).await.unwrap();

        dbg!(&res);
    }

    pub async fn later_block(&self, zilliqa: &Zilliqa) -> Result<u64, Error> {
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
}
