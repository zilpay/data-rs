use bytes::Bytes;
use http_body_util::Full;
use hyper::{
    header::{self, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN},
    http::HeaderValue,
    Request, Response,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::models::currencies::Currencies;

pub async fn handle_get_rates(
    _req: Request<hyper::body::Incoming>,
    rates: Arc<RwLock<Currencies>>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    // TODO: add currency query.
    let json = rates.read().await.serializatio();
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
