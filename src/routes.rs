use hyper::client::HttpConnector;
use hyper::{Body, Client, Request, Response, StatusCode};
use hyper_rustls::HttpsConnector;
use tokio::fs::File;

use crate::file;
use crate::redir::{Action, Rules};

pub struct State {
    client: Client<HttpsConnector<HttpConnector>>,
    rules: Rules,
}

impl State {
    pub fn new(client: Client<HttpsConnector<HttpConnector>>, rules: Rules) -> Self {
        Self { client, rules }
    }
}

pub async fn respond_to_request(
    mut req: Request<Body>,
    state: &State,
) -> Result<Response<Body>, hyper::Error> {
    match state.rules.try_match(req.uri()) {
        Some(Ok(Action::Http(uri))) => {
            log::info!("{} -> {}", req.uri(), uri);
            *req.uri_mut() = uri;
            state.client.request(req).await
        }
        Some(Ok(Action::File { path, fallback })) => {
            let found_file = match File::open(&path).await {
                Ok(file) => Ok((path, file)),
                Err(e) => match fallback {
                    Some(fallback) => match File::open(&fallback).await {
                        Ok(file) => Ok((fallback, file)),
                        Err(_) => Err((path, e)),
                    },
                    None => Err((path, e)),
                },
            };
            match found_file {
                Ok((found_path, file)) => {
                    log::info!("{} -> {}", req.uri(), found_path.display());
                    Ok(Response::new(file::body_stream(file)))
                }
                Err((path, e)) => {
                    log::warn!("{} -> [file error] {} : {}", req.uri(), path.display(), e);
                    let mut resp = Response::new(Body::empty());
                    *resp.status_mut() = StatusCode::NOT_FOUND;
                    Ok(resp)
                }
            }
        }
        Some(Ok(Action::Status(status))) => {
            log::info!("{} -> {}", req.uri(), status);
            let mut resp = Response::new(Body::empty());
            *resp.status_mut() = status;
            Ok(resp)
        }
        Some(Err(e)) => {
            log::warn!("{} -> [internal error] : {}", req.uri(), e);
            let mut resp = Response::new(Body::empty());
            *resp.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
            Ok(resp)
        }
        None => {
            log::warn!("{} -> [no match]", req.uri());
            let mut resp = Response::new(Body::empty());
            *resp.status_mut() = StatusCode::BAD_GATEWAY;
            Ok(resp)
        }
    }
}
