use log::{error, info};
use serde::Deserialize;
use serde_json::json;
use sled::{Db, IVec};
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::u128;

use crate::{
    config::{
        dex::{DEX, DEX_KEY},
        zilliqa::RPC_METHODS,
    },
    utils::zilliqa::{JsonBodyReq, JsonBodyRes, Zilliqa},
};

#[derive(Debug)]
pub struct Dex {
    pub pools: HashMap<String, (u128, u128)>,
    db: Db,
    app_name: &'static str,
}

#[derive(Debug, Deserialize)]
struct Pool {
    arguments: (String, String),
}

#[derive(Debug, Deserialize)]
struct ResPoolState {
    pools: HashMap<String, Pool>,
}

impl Dex {
    pub fn new(db_path: &str) -> Self {
        let app_name = "DEX";
        let db = sled::open(db_path).expect("Cannot dex open database.");
        let pools: HashMap<String, (u128, u128)> = match db.get(DEX_KEY) {
            Ok(mb_cache) => {
                let cache = mb_cache.unwrap_or(IVec::default());
                let mb_json = std::str::from_utf8(&cache).unwrap();
                let pools = serde_json::from_str(mb_json).unwrap_or(HashMap::new());

                info!("{app_name}: loaded from cache {}", pools.len());

                pools
            }
            Err(_) => {
                error!("{app_name}: fail to load cache!");

                HashMap::new()
            }
        };

        Dex {
            db,
            pools,
            app_name,
        }
    }

    pub fn update(&mut self, pools: HashMap<String, (u128, u128)>) -> Result<(), Error> {
        self.pools = pools;
        self.db.insert(DEX_KEY, self.serializatio().as_bytes())?;

        info!("{:?}: updated pools {:?}", self.app_name, self.pools.len());

        Ok(())
    }

    pub async fn get_pools(zilliqa: &Zilliqa) -> Result<HashMap<String, (u128, u128)>, Error> {
        let pools = Dex::fetch(zilliqa).await?;

        Ok(pools)
    }

    pub fn serializatio(&self) -> String {
        serde_json::to_string(&self.pools).unwrap()
    }

    async fn fetch(zilliqa: &Zilliqa) -> Result<HashMap<String, (u128, u128)>, Error> {
        let field = "pools";
        let custom_error = Error::new(ErrorKind::Other, "Fail to fetch or parse response");
        let params = json!([DEX, field, []]);
        let bodies: Vec<JsonBodyReq> =
            vec![zilliqa.build_body(RPC_METHODS.get_smart_contract_sub_state, params)];
        let res: Vec<JsonBodyRes<ResPoolState>> = zilliqa.fetch(bodies).await?;
        let pools = match res.get(0) {
            Some(res) => match &res.result {
                Some(result) => &result.pools,
                None => return Err(custom_error),
            },
            None => {
                return Err(custom_error);
            }
        };
        let pools: HashMap<String, (u128, u128)> = pools
            .into_iter()
            .filter_map(|(key, value)| {
                let key = key.to_string();
                let zils: u128 = value.arguments.0.parse().unwrap();
                let tokens: u128 = value.arguments.1.parse().unwrap();
                let args = (zils, tokens);

                if zils == 0 || tokens == 0 {
                    return None;
                }

                Some((key, args))
            })
            .collect();

        Ok(pools)
    }
}
