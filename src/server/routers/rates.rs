use bytes::Bytes;
use http_body_util::Full;
use hyper::{header, Request, Response};
use std::sync::{Arc, Mutex};

use crate::models::currencies::Currencies;

pub async fn handle_get_rates(
    _req: Request<hyper::body::Incoming>,
    rates_ref: Arc<Mutex<Currencies>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let rates = rates_ref.lock().unwrap();
    let json = rates.serializatio();
    let response = Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(json)))
        .unwrap();

    Ok(response)
}
