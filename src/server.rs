use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Client, Server};
use hyper_rustls::HttpsConnector;
use tokio::runtime;

use crate::err::Error;
use crate::redir::Rules;
use crate::routes::{respond_to_request, State};

pub fn run(addr: &SocketAddr, rules: Rules) -> Result<(), Error> {
    let mut runtime = runtime::Builder::new()
        .basic_scheduler()
        .enable_all()
        .build()?;

    let client = Client::builder().build(HttpsConnector::new());

    let state = Arc::new(State::new(client, rules));
    let make_svc = make_service_fn(move |_| {
        let state = Arc::clone(&state);
        let svc = service_fn(move |req| {
            let state = Arc::clone(&state);
            async move { respond_to_request(req, &state).await }
        });
        async move { Ok::<_, Infallible>(svc) }
    });

    let server = runtime.enter(|| Server::try_bind(&addr))?.serve(make_svc);

    runtime.block_on(server)?;

    Ok(())
}
