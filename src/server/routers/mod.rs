use bytes::Bytes;
use http_body_util::Full;
use hyper::StatusCode;
use hyper::{Request, Response};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::{currencies::Currencies, dex::Dex, meta::Meta};

mod meta;
mod rates;

pub async fn route(
    req: Request<hyper::body::Incoming>,
    meta: Arc<RwLock<Meta>>,
    dex: Arc<RwLock<Dex>>,
    rates: Arc<RwLock<Currencies>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/api/v1/rates") => rates::handle_get_rates(req, rates).await,
        (&hyper::Method::POST, "/") => meta::handle_get_meta(req).await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("Not Found")))
            .unwrap()),
    }
}
