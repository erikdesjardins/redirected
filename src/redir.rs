use std::path::PathBuf;
use std::str::FromStr;

use failure::Fail;
use http::uri::InvalidUri;
use hyper::Uri;

use crate::util::IntoOptionExt;

#[derive(Debug)]
pub struct From(String);

#[derive(Debug, Fail)]
pub enum BadRedirectFrom {
    #[fail(display = "path does not start with slash")]
    NoLeadingSlash,
    #[fail(display = "path does not end with slash")]
    NoTrailingSlash,
}

impl FromStr for From {
    type Err = BadRedirectFrom;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match () {
            _ if !s.starts_with('/') => Err(BadRedirectFrom::NoLeadingSlash),
            _ if !s.ends_with('/') => Err(BadRedirectFrom::NoTrailingSlash),
            _ => Ok(From(s.to_string())),
        }
    }
}

#[derive(Debug)]
pub enum To {
    Http(String),
    File(PathBuf),
}

#[derive(Debug, Fail)]
pub enum BadRedirectTo {
    #[fail(display = "invalid uri: {}", _0)]
    InvalidUri(InvalidUri),
    #[fail(display = "invalid scheme: {}", _0)]
    InvalidScheme(String),
    #[fail(display = "uri does not end with slash")]
    NoTrailingSlash,
    #[fail(display = "uri does not begin with scheme")]
    NoScheme,
}

impl FromStr for To {
    type Err = BadRedirectTo;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<Uri>() {
            Ok(uri) => match uri.scheme_part().map(|s| s.as_str()) {
                Some("http") | Some("https") => {
                    let uri = uri.to_string();
                    match () {
                        _ if !uri.ends_with('/') => Err(BadRedirectTo::NoTrailingSlash),
                        _ => Ok(To::Http(uri)),
                    }
                }
                Some("file") => {
                    let uri =
                        uri.authority_part().map_or("", |a| a.as_str()).to_string() + uri.path();
                    match () {
                        _ if !uri.ends_with('/') => Err(BadRedirectTo::NoTrailingSlash),
                        _ => Ok(To::File(PathBuf::from(uri))),
                    }
                }
                Some(scheme) => Err(BadRedirectTo::InvalidScheme(scheme.to_string())),
                None => Err(BadRedirectTo::NoScheme),
            },
            Err(e) => Err(BadRedirectTo::InvalidUri(e)),
        }
    }
}

#[derive(Debug, Fail)]
pub enum BadRedirect {
    #[fail(display = "unequal number of `from` and `to` arguments")]
    UnequalFromTo,
}

pub struct Rules {
    redirects: Vec<(From, To)>,
}

impl Rules {
    pub fn zip(from: Vec<From>, to: Vec<To>) -> Result<Self, BadRedirect> {
        if from.len() == to.len() {
            Ok(Self {
                redirects: from.into_iter().zip(to).collect(),
            })
        } else {
            Err(BadRedirect::UnequalFromTo)
        }
    }

    pub fn try_match(&self, uri: &Uri) -> Option<Result<Action, InvalidUri>> {
        let req_path = uri.path_and_query()?.as_str();
        self.redirects.iter().find_map(|(from, to)| {
            req_path
                .trim_start_matches(from.0.as_str())
                .some_if(|&t| t != req_path)
                .map(|req_tail| {
                    Ok(match to {
                        To::Http(prefix) => Action::Http((prefix.to_string() + req_tail).parse()?),
                        To::File(prefix) => Action::File(prefix.join(req_tail)),
                    })
                })
        })
    }
}

pub enum Action {
    Http(Uri),
    File(PathBuf),
}
