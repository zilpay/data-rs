use bytes::Bytes;
use http_body_util::Full;
use hyper::StatusCode;
use hyper::{Request, Response};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::shit_wallet::ShitWallet;
use crate::models::{currencies::Currencies, dex::Dex, meta::Meta};

mod dex;
mod rates;
mod tokens;
mod wallet;

pub async fn route(
    req: Request<hyper::body::Incoming>,
    meta: Arc<RwLock<Meta>>,
    dex: Arc<RwLock<Dex>>,
    rates: Arc<RwLock<Currencies>>,
    shit_wallet: Arc<RwLock<ShitWallet>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/api/v1/dex") => dex::handle_get_pools(req, meta, dex, rates).await,
        (&hyper::Method::GET, "/api/v1/rates") => rates::handle_get_rates(req, rates).await,
        (&hyper::Method::GET, "/api/v1/tokens") => tokens::handle_get_tokens(req, meta).await,
        (&hyper::Method::GET, path) if path.starts_with("/api/v1/token/") => {
            tokens::handle_get_token(req, meta).await
        }
        (&hyper::Method::PUT, path) if path.starts_with("/api/v1/token/") => {
            tokens::handle_update_token(req, meta).await
        }
        (&hyper::Method::GET, "/api/v1/shits") => {
            wallet::handle_get_wallets(req, shit_wallet).await
        }
        (&hyper::Method::GET, path) if path.starts_with("/api/v1/shit/") => {
            wallet::handle_get_a_wallet(req, shit_wallet).await
        }
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("Not Found")))
            .unwrap()),
    }
}
