use bytes::Bytes;
use http_body_util::Full;
use hyper::StatusCode;
use hyper::{Request, Response};

mod meta;
mod rates;

pub async fn route(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&hyper::Method::GET, "/") => rates::handle_get_rates(req).await,
        (&hyper::Method::POST, "/") => meta::handle_get_meta(req).await,
        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("Not Found")))
            .unwrap()),
    }
}
