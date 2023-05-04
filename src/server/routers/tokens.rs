use crate::models::meta::Meta;
use bytes::Bytes;
use http_body_util::BodyExt;
use http_body_util::Full;
use hyper::{header, Request, Response, StatusCode};
use serde_json::map::Values;
use serde_json::Value;
use serde_json::{self, json};
use std::iter::Map;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    let response = Response::builder().header(header::CONTENT_TYPE, "application/json");
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
            let response = Response::builder()
                .header(header::CONTENT_TYPE, "application/json")
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

    return Ok(response);
}
