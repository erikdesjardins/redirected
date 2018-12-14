use std::net::SocketAddr;
use std::sync::Arc;

use failure::Error;
use futures::future::Either::{A, B};
use futures::{future, Future};
use hyper::service::service_fn;
use hyper::{Body, Client, Response, Server, StatusCode};
use log::{info, warn};
use tokio::runtime::Runtime;

use crate::file;
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
                A(A(client.request(req)))
            }
            Some(Ok(Action::File(path))) => A(B(file::resolve_or_index(path.clone()).then(
                move |r| match r {
                    Ok(file) => {
                        info!("{} -> {}", req.uri(), path.display());
                        Ok(Response::new(file::body_stream(file)))
                    }
                    Err(e) => {
                        warn!("{} -> [{}]", req.uri(), e);
                        let mut resp = Response::new(Body::empty());
                        *resp.status_mut() = StatusCode::NOT_FOUND;
                        Ok(resp)
                    }
                },
            ))),
            Some(Err(e)) => {
                warn!("{} -> [{}]", req.uri(), e);
                let mut resp = Response::new(Body::empty());
                *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                B(future::ok(resp))
            }
            None => {
                warn!("{} -> [no match]", req.uri());
                let mut resp = Response::new(Body::empty());
                *resp.status_mut() = StatusCode::BAD_GATEWAY;
                B(future::ok(resp))
            }
        })
    });

    runtime.block_on(server)?;

    Ok(())
}
