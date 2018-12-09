use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use failure::{Error, ResultExt};
use futures::future;
use hyper::service::service_fn;
use hyper::{Body, Client, Response, Server, StatusCode};
use log::{info, warn};
use tokio::runtime::Runtime;

use crate::redir::Redir;

pub fn run(mappings: HashMap<SocketAddr, Vec<Redir>>) -> Result<(), Error> {
    let mut runtime = Runtime::new()?;

    let client = Client::new();

    let servers = mappings
        .into_iter()
        .map(move |(addr, rules)| -> Result<_, Error> {
            let client = client.clone();
            let rules = Arc::new(rules);

            let server = Server::try_bind(&addr)
                .with_context(|_| format!("Failed to bind to {}", addr))?
                .serve(move || {
                    let client = client.clone();
                    let rules = rules.clone();

                    service_fn(move |mut req| {
                        let redir_uri = req
                            .uri()
                            .path_and_query()
                            .map(|p| p.as_str())
                            .and_then(|path_and_query| {
                                rules.iter().find_map(|rule| {
                                    if path_and_query.starts_with(&rule.from) {
                                        Some(rule.to.clone() + &path_and_query[rule.from.len()..])
                                    } else {
                                        None
                                    }
                                })
                            }).map(|u| u.parse());

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
            Ok(server)
        }).collect::<Result<Vec<_>, _>>()?;

    runtime.block_on(future::join_all(servers))?;

    Ok(())
}
