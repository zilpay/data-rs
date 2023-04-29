use hyper::server::conn::http1;
use hyper::service::service_fn;
use log::{error, info};
use std::{io, net::SocketAddr};
use tokio::net::TcpListener;

use routers::route;

use crate::{
    config::server::PORT,
    models::{currencies::Currencies, dex::Dex, meta::Meta},
};

mod routers;

pub async fn run_server(
    meta: &'static Meta,
    dex: &'static Dex,
    rates: &'static Currencies,
) -> Result<(), io::Error> {
    let addr = SocketAddr::from(([127, 0, 0, 1], PORT));
    let listener = TcpListener::bind(&addr).await.unwrap();

    info!("Listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;

        tokio::task::spawn(async move {
            let service = service_fn(move |req| route(req, meta, dex, rates));

            if let Err(err) = http1::Builder::new()
                .serve_connection(stream, service)
                .await
            {
                error!("Failed to serve connection: {:?}", err);
            }
        });
    }
}
