use bytes::Bytes;
use http_body_util::Full;
use hyper::{Request, Response};

pub async fn handle_get_rates(
    _req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let response = Response::builder()
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Full::new(Bytes::from("Hello from the GET handler!")))
        .unwrap();

    Ok(response)
}
