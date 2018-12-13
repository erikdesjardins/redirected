use std::net::SocketAddr;
use std::sync::Arc;

use failure::Error;
use futures::future;
use hyper::service::service_fn;
use hyper::{Body, Client, Response, Server, StatusCode};
use log::{info, warn};
use tokio::runtime::Runtime;

use crate::redir::{Action, Rules};

pub fn run(addr: &SocketAddr, rules: Rules) -> Result<(), Error> {
    let mut runtime = Runtime::new()?;

    let client = Client::new();
    let rules = Arc::new(rules);

    let server = Server::try_bind(&addr)?.serve(move || {
        let client = client.clone();
        let rules = rules.clone();

        service_fn(move |mut req| match rules.try_match(req.uri()) {
            Some(Ok(Action::Http(uri))) => {
                info!("{} -> {}", req.uri(), uri);
                *req.uri_mut() = uri;
                future::Either::A(client.request(req))
            }
            Some(Err(e)) => {
                warn!("{} -> <internal error>: {}", req.uri(), e);
                let mut resp = Response::new(Body::empty());
                *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                future::Either::B(future::ok(resp))
            }
            None => {
                warn!("{} -> <no match>", req.uri());
                let mut resp = Response::new(Body::empty());
                *resp.status_mut() = StatusCode::BAD_GATEWAY;
                future::Either::B(future::ok(resp))
            }
        })
    });

    runtime.block_on(server)?;

    Ok(())
}
