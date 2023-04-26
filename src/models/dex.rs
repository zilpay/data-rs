use log::{error, info, LevelFilter};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use simple_logger::SimpleLogger;
use sled::{Db, IVec};
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::u128;

use crate::{
    config::{
        dex::{DEX, DEX_DATABASE, DEX_KEY},
        zilliqa::RPC_METHODS,
    },
    utils::zilliqa::{JsonBodyReq, JsonBodyRes, Zilliqa},
};

#[derive(Debug)]
pub struct Dex {
    pub pools: HashMap<String, (String, String)>,
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
    pub fn new() -> Self {
        SimpleLogger::new()
            .with_colors(true)
            .with_level(LevelFilter::Info)
            .init()
            .unwrap();

        let app_name = "DEX";
        let db = sled::open(DEX_DATABASE).expect("Cannot dex open database.");
        let pools: HashMap<String, (String, String)> = match db.get(DEX_KEY) {
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

    pub async fn update(&self, zilliqa: &Zilliqa) {
        self.fetch(zilliqa).await;
    }

    pub fn serializatio(&self) -> String {
        serde_json::to_string(&self.pools).unwrap()
    }

    async fn fetch(
        &self,
        zilliqa: &Zilliqa,
    ) -> Result<HashMap<String, (String, String)>, std::io::Error> {
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
        let pools: HashMap<String, (String, String)> = pools
            .into_iter()
            .map(|(key, value)| {
                (
                    key.to_string(),
                    (value.arguments.0.to_string(), value.arguments.1.to_string()),
                )
            })
            .collect();

        Ok(pools)
    }
}
