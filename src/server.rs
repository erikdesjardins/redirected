use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use failure::Error;
use futures::{future, Future};
use hyper::service::service_fn;
use hyper::{rt, Body, Chunk, Client, Response, Server, StatusCode};
use log::{error, info, warn};

use redir::Redir;

pub fn run(mappings: HashMap<SocketAddr, Vec<Redir>>) -> Result<(), Error> {
    let client = Client::new();

    let servers = mappings.into_iter().map(move |(addr, rules)| {
        let client = client.clone();
        let rules = Arc::new(rules);
        info!("Binding to {}", addr);
        Server::bind(&addr).serve(move || {
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
                        info!("{}", uri);
                        *req.uri_mut() = uri;
                        future::Either::A(client.request(req))
                    }
                    Some(Err(e)) => {
                        warn!("Invalid internal uri: {}", e);
                        future::Either::B(future::ok(
                            Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(Chunk::from(e.to_string())))
                                .expect("trivial builder usage"),
                        ))
                    }
                    None => {
                        warn!("No matches found");
                        future::Either::B(future::ok(
                            Response::builder()
                                .status(StatusCode::BAD_GATEWAY)
                                .body(Body::from(Chunk::from(
                                    "request matched no redirect rules".to_string(),
                                ))).expect("trivial builder usage"),
                        ))
                    }
                }
            })
        })
    });

    rt::run(
        future::join_all(servers)
            .map(|_| ())
            .map_err(|e| error!("{}", e)),
    );

    Ok(())
}
