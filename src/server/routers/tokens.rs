use crate::models::meta::Meta;
use crate::models::meta::Token;
use bytes::Bytes;
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::{header, Request, Response, StatusCode};
use serde_json::Value;
use serde_json::{self, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::dex::ListedTokens;

pub async fn handle_get_tokens(
    req: Request<hyper::body::Incoming>,
    meta: Arc<RwLock<Meta>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut params_map = HashMap::new();
    let query_params = req.uri().query().unwrap_or("");
    let parsed_params = url::form_urlencoded::parse(query_params.as_bytes());

    for (key, value) in parsed_params {
        params_map.insert(key.into_owned(), value.into_owned());
    }

    let limit: usize = params_map
        .get("limit")
        .unwrap_or(&"20".to_string())
        .parse()
        .unwrap_or(20);
    let token_type: u8 = params_map
        .get("type")
        .unwrap_or(&"1".to_string())
        .parse()
        .unwrap_or(1);
    let offset: usize = params_map
        .get("offset")
        .unwrap_or(&"0".to_string())
        .parse()
        .unwrap_or(0);

    for token in meta.read().await.list.iter() {
        if token.token_type == token_type && token.status == 1 {
            tokens.push(token.clone());
        }
    }

    let tokens = tokens
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<Token>>();
    let tokens_res = ListedTokens {
        count: tokens.len(),
        list: tokens,
    };

    let res_json = serde_json::to_string(&tokens_res).unwrap();
    let response = Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(res_json)))
        .unwrap();

    Ok(response)
}

pub async fn handle_get_token(
    req: Request<hyper::body::Incoming>,
    meta: Arc<RwLock<Meta>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let params = req.uri().path().split("/").collect::<Vec<&str>>();
    let symbol = params.last().unwrap_or(&"").clone().to_lowercase();

    if let Some(token) = meta
        .read()
        .await
        .list
        .iter()
        .find(|t| t.symbol.to_lowercase() == symbol && t.status == 1)
    {
        let res_json = serde_json::to_string(&token).unwrap();
        let response = Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(res_json)))
            .unwrap();

        Ok(response)
    } else {
        let res = json!({
            "code": -1,
            "message": format!("No token {}", symbol)
        });
        let not_found = serde_json::to_string(&res).unwrap();
        let response = Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from(not_found)))
            .unwrap();

        Ok(response)
    }
}

pub async fn handle_update_token(
    req: Request<hyper::body::Incoming>,
    meta: Arc<RwLock<Meta>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let access_token = std::env::var("ACCESS_TOKEN").unwrap_or("666".to_string());
    let header_token = match req.headers().get("Authorization") {
        Some(value) => value.to_str().unwrap_or(""),
        None => "",
    };
    let response = Response::builder().header(header::CONTENT_TYPE, "application/json");

    if access_token != header_token {
        let res = json!({
            "code": -5,
            "message": "Incorrect atuh token"
        });
        let res_json = serde_json::to_string(&res).unwrap();
        let response = response
            .status(StatusCode::NETWORK_AUTHENTICATION_REQUIRED)
            .body(Full::new(Bytes::from(res_json)))
            .unwrap();

        return Ok(response);
    }

    let params = req.uri().path().split("/").collect::<Vec<&str>>();
    let symbol = params.last().unwrap_or(&"").clone().to_lowercase();
    let body_bytes = req.collect().await?.to_bytes();
    let value: Value = match serde_json::from_slice(&body_bytes) {
        Ok(v) => v,
        Err(_) => {
            let res = json!({
                "code": -2,
                "message": "Incorrect params"
            });
            let res_json = serde_json::to_string(&res).unwrap();
            let response = response
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(res_json)))
                .unwrap();

            return Ok(response);
        }
    };
    let map = match value.as_object() {
        Some(v) => v,
        None => {
            let res = json!({
                "code": -2,
                "message": "Incorrect params"
            });
            let res_json = serde_json::to_string(&res).unwrap();
            let response = response
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(res_json)))
                .unwrap();

            return Ok(response);
        }
    };
    let status = map.get("status");
    let score = map.get("score");
    let listed = map.get("listed");
    let mut token_meta = meta.write().await;
    let token_index = match token_meta
        .list
        .iter()
        .position(|t| t.symbol.to_lowercase() == symbol)
    {
        Some(index) => index,
        None => {
            let res = json!({
                "code": -1,
                "message": format!("No token {}", symbol)
            });
            let not_found = serde_json::to_string(&res).unwrap();
            let response = response
                .status(StatusCode::NOT_FOUND)
                .body(Full::new(Bytes::from(not_found)))
                .unwrap();

            return Ok(response);
        }
    };

    if let Some(status) = status {
        let new_status = status.as_u64().unwrap_or(0);
        let new_status: u8 = if new_status > 1 { 1 } else { new_status as u8 };

        token_meta.list[token_index].status = new_status;
    }
    if let Some(score) = score {
        let new_score = score.as_u64().unwrap_or(0);
        let new_score: u8 = if new_score > 100 {
            100
        } else {
            new_score as u8
        };

        token_meta.list[token_index].score = new_score;
    }
    if let Some(listed) = listed {
        let new_listed = listed.as_bool().unwrap_or(false);

        token_meta.list[token_index].listed = new_listed;
    }

    match token_meta.write_db() {
        Ok(_) => (),
        Err(_) => {
            let res = json!({
                "code": -4,
                "message": "Cannot write database"
            });
            let wr_db = serde_json::to_string(&res).unwrap();
            let response = response
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from(wr_db)))
                .unwrap();

            return Ok(response);
        }
    };

    let res = json!({ "message": format!("updated token {}", symbol) });
    let ok = serde_json::to_string(&res).unwrap();
    let response = response
        .status(StatusCode::OK)
        .body(Full::new(Bytes::from(ok)))
        .unwrap();

    return Ok(response);
}
