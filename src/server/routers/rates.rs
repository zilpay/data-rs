use bytes::Bytes;
use http_body_util::Full;
use hyper::{header, Request, Response};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::currencies::Currencies;

pub async fn handle_get_rates(
    _req: Request<hyper::body::Incoming>,
    rates: Arc<RwLock<Currencies>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let json = rates.read().await.serializatio();
    let response = Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap();

    Ok(response)
}
