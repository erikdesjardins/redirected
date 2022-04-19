use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Client, Server};
use hyper_rustls::HttpsConnectorBuilder;

use crate::err::Error;
use crate::redir::Rules;
use crate::routes::{respond_to_request, State};

pub async fn run(addr: &SocketAddr, rules: Rules) -> Result<(), Error> {
    let client = Client::builder().build(
        HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_or_http()
            .enable_http1()
            .build(),
    );

    let state = Arc::new(State::new(client, rules));
    let make_svc = make_service_fn(move |_| {
        let state = Arc::clone(&state);
        let svc = service_fn(move |req| {
            let state = Arc::clone(&state);
            async move { respond_to_request(req, &state).await }
        });
        async move { Ok::<_, Infallible>(svc) }
    });

    Server::try_bind(addr)?.serve(make_svc).await?;

    Ok(())
}
