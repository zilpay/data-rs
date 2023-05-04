use hyper::server::conn::http1;
use hyper::service::service_fn;
use log::{error, info};
use std::{io, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use routers::route;

use crate::{
    config::server::PORT,
    models::{currencies::Currencies, dex::Dex, meta::Meta},
};

mod routers;

pub async fn run_server(
    meta: &Arc<RwLock<Meta>>,
    dex: &Arc<RwLock<Dex>>,
    rates: &Arc<RwLock<Currencies>>,
) -> Result<(), io::Error> {
    let addr = SocketAddr::from(([0, 0, 0, 0], PORT));
    let listener = TcpListener::bind(&addr).await.unwrap();

    info!("Listening on http://{}", addr);

    loop {
        let (stream, _) = listener.accept().await?;
        let meta_ref = Arc::clone(&meta);
        let dex_ref = Arc::clone(&dex);
        let rates_ref = Arc::clone(&rates);

        tokio::task::spawn(async move {
            let service = service_fn(move |req| {
                route(req, meta_ref.clone(), dex_ref.clone(), rates_ref.clone())
            });

            if let Err(err) = http1::Builder::new()
                .serve_connection(stream, service)
                .await
            {
                error!("Failed to serve connection: {:?}", err);
            }
        });
    }
}
