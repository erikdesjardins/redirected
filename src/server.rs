use std::net::SocketAddr;
use std::sync::Arc;

use failure::Error;
use futures::future;
use hyper::service::service_fn;
use hyper::{Body, Client, Response, Server, StatusCode};
use log::{info, warn};
use tokio::runtime::Runtime;

use crate::redir::Redirect;

pub fn run(addr: &SocketAddr, redirects: Vec<Redirect>) -> Result<(), Error> {
    let mut runtime = Runtime::new()?;

    let client = Client::new();
    let redirects = Arc::new(redirects);

    let server = Server::try_bind(&addr)?.serve(move || {
        let client = client.clone();
        let redirects = redirects.clone();

        service_fn(move |mut req| {
            let redir_uri = req
                .uri()
                .path_and_query()
                .map(|p| p.as_str())
                .and_then(|path_and_query| {
                    redirects.iter().find_map(|rule| {
                        if path_and_query.starts_with(&*rule.from) {
                            Some(rule.to.to_string() + &path_and_query[rule.from.len()..])
                        } else {
                            None
                        }
                    })
                })
                .map(|u| u.parse());

            match redir_uri {
                Some(Ok(uri)) => {
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
            }
        })
    });

    runtime.block_on(server)?;

    Ok(())
}
