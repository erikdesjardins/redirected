use std::net::SocketAddr;
use std::sync::Arc;

use futures::future::Either::{A, B};
use futures::{future, Future};
use hyper::service::service_fn;
use hyper::{Body, Client, Response, Server, StatusCode};
use hyper_tls::HttpsConnector;
use tokio::fs::File;
use tokio::runtime::Runtime;

use crate::err::Error;
use crate::file;
use crate::redir::{Action, Rules};

pub fn run(addr: &SocketAddr, rules: Rules) -> Result<(), Error> {
    let mut runtime = Runtime::new()?;

    let https_connector = HttpsConnector::new(4)?;
    let client = Client::builder().build(https_connector);
    let rules = Arc::new(rules);

    let server = Server::try_bind(&addr)?.serve(move || {
        let client = client.clone();
        let rules = rules.clone();

        service_fn(move |mut req| match rules.try_match(req.uri()) {
            Some(Ok(Action::Http(uri))) => {
                log::info!("{} -> {}", req.uri(), uri);
                *req.uri_mut() = uri;
                A(A(client.request(req)))
            }
            Some(Ok(Action::File { path, fallback })) => A(B({
                let main = path.clone();
                let index = path.join("index.html");
                future::err(())
                    .or_else(|_| File::open(main.clone()).map(|f| (main, f)))
                    .or_else(|_| File::open(index.clone()).map(|f| (index, f)))
                    .or_else(|e| match fallback {
                        Some(fallback) => A(File::open(fallback.clone()).map(|f| (fallback, f))),
                        None => B(future::err(e)),
                    })
                    .then(move |r| match r {
                        Ok((resolved, file)) => {
                            log::info!("{} -> {}", req.uri(), resolved.display());
                            Ok(Response::new(file::body_stream(file)))
                        }
                        Err(e) => {
                            log::warn!(
                                "{} -> [file error] {} (or index/fallback) : {}",
                                req.uri(),
                                path.display(),
                                e
                            );
                            let mut resp = Response::new(Body::empty());
                            *resp.status_mut() = StatusCode::NOT_FOUND;
                            Ok(resp)
                        }
                    })
            })),
            Some(Ok(Action::Status(status))) => {
                log::info!("{} -> {}", req.uri(), status);
                let mut resp = Response::new(Body::empty());
                *resp.status_mut() = status;
                B(future::ok(resp))
            }
            Some(Err(e)) => {
                log::warn!("{} -> [internal error] : {}", req.uri(), e);
                let mut resp = Response::new(Body::empty());
                *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                B(future::ok(resp))
            }
            None => {
                log::warn!("{} -> [no match]", req.uri());
                let mut resp = Response::new(Body::empty());
                *resp.status_mut() = StatusCode::BAD_GATEWAY;
                B(future::ok(resp))
            }
        })
    });

    runtime.block_on(server)?;

    Ok(())
}
