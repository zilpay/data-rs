use hyper::server::conn::http1;
use hyper::service::service_fn;
use log::{error, info};
use std::{io, net::SocketAddr};
use tokio::net::TcpListener;

use routers::route;

mod routers;

pub async fn run_server() -> Result<(), io::Error> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = TcpListener::bind(&addr).await.unwrap();

    info!("Listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::task::spawn(async move {
            let service = service_fn(move |req| route(req));

            if let Err(err) = http1::Builder::new()
                .serve_connection(stream, service)
                .await
            {
                error!("Failed to serve connection: {:?}", err);
            }
        });
    }
}
