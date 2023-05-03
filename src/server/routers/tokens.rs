use crate::models::meta::Meta;
use bytes::Bytes;
use http_body_util::Full;
use hyper::{header, Request, Response, StatusCode};
use serde_json::{self, json};
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
