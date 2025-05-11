use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use hyper_util::server::conn::auto;
use log::{error, info};
use std::{io, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tokio::sync::RwLock;

use routers::route;

use crate::models::{currencies::Currencies, dex::Dex, meta::Meta};

mod routers;

pub async fn run_server(
    meta: &Arc<RwLock<Meta>>,
    dex: &Arc<RwLock<Dex>>,
    rates: &Arc<RwLock<Currencies>>,
    port: u16,
) -> Result<(), io::Error> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(&addr).await?;

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

            let io = TokioIo::new(stream);

            if let Err(err) = auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                .serve_connection(io, service)
                .await
            {
                error!("Failed to serve connection: {:?}", err);
            }
        });
    }
}
