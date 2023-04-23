// use data_rs::models::currencies::Currencies;
use data_rs::{
    config::zilliqa::RPC_METHODS,
    models::meta::Meta,
    utils::zilliqa::{JsonBodyReq, JsonBodyRes, Zilliqa},
};
use serde_json::{Map, Value};
use tokio;

#[tokio::main]
async fn main() {
    let zil = Zilliqa::new();
    let mut bodies: Vec<JsonBodyReq> = Vec::new();
    let params = vec![String::from("07da3d45b0f1390083097a95a8915fc2f6b06c6f")];

    bodies.push(zil.build_body(RPC_METHODS.get_smart_contract_init, params.clone()));

    let res: Vec<JsonBodyRes<Vec<Map<String, Value>>>> = zil.fetch(bodies).await.unwrap();

    dbg!(res);

    // let rates = Currencies::new();
    // let mut meta = Meta::new();

    // meta.update().await;

    // dbg!(meta.serializatio());

    // rates.update().await;
}
