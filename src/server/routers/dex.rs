use bytes::Bytes;
use http_body_util::Full;
use hyper::{header, Request, Response};
use serde::Serialize;
use serde_json::{self, json};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use crate::models::{
    currencies::Currencies,
    dex::Dex,
    meta::{Meta, Token},
};

#[derive(Debug, Serialize)]
struct ListedTokens {
    count: usize,
    list: Vec<Token>,
}

#[derive(Debug, Serialize)]
struct DexResponse {
    tokens: ListedTokens,
    pools: HashMap<String, (String, String)>,
    rate: String,
}

pub async fn handle_get_pools(
    _req: Request<hyper::body::Incoming>,
    meta: Arc<RwLock<Meta>>,
    dex: Arc<RwLock<Dex>>,
    rates: Arc<RwLock<Currencies>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut pools: HashMap<String, (String, String)> = HashMap::new();
    let count = pools.len();
    let rate = rates
        .read()
        .await
        .data
        .get("usd")
        .unwrap_or(&json!("0"))
        .to_string();

    for token in meta.read().await.list.iter() {
        if token.listed && token.token_type == 1 {
            tokens.push(token.clone());
        }
    }

    for (key, values) in dex.read().await.pools.iter() {
        pools.insert(
            key.to_string(),
            (values.0.to_string(), values.1.to_string()),
        );
    }

    let tokens_res = ListedTokens {
        count,
        list: tokens,
    };
    let response = DexResponse {
        rate,
        pools,
        tokens: tokens_res,
    };
    let json_str = serde_json::to_string(&response).unwrap();
    let response = Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(json_str)))
        .unwrap();

    Ok(response)
}
