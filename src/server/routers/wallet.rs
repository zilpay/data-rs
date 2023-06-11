use bytes::Bytes;
use http_body_util::Full;
use hyper::{
    header::{self, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN},
    http::HeaderValue,
    Request, Response, StatusCode,
};
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

use crate::models::shit_wallet::ShitWallet;

pub async fn handle_get_wallets(
    _req: Request<hyper::body::Incoming>,
    wallets: Arc<RwLock<ShitWallet>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let json = wallets.read().await.wallet_serializatio();
    let mut response = Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap();

    response
        .headers_mut()
        .insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    response.headers_mut().insert(
        ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET"),
    );

    Ok(response)
}

pub async fn handle_get_a_wallet(
    req: Request<hyper::body::Incoming>,
    wallets: Arc<RwLock<ShitWallet>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let params = req.uri().path().split("/").collect::<Vec<&str>>();
    let addr = params.last().unwrap_or(&"").clone().to_lowercase();

    if let Some(wallet) = wallets.read().await.wallets.get(&addr.to_lowercase()) {
        let res_json = serde_json::to_string(&wallet).unwrap();
        let response = Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(Full::new(Bytes::from(res_json)))
            .unwrap();

        Ok(response)
    } else {
        let res = json!({
            "code": -1,
            "message": format!("no found smart wallet for {}", addr)
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
