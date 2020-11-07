use std::path::PathBuf;
use std::str::FromStr;

use http::status::InvalidStatusCode;
use http::uri::InvalidUri;
use hyper::{StatusCode, Uri};
use thiserror::Error;

#[derive(Debug)]
pub struct From(String);

#[derive(Debug, Error)]
pub enum BadRedirectFrom {
    #[error("path does not start with slash")]
    NoLeadingSlash,
    #[error("path does not end with slash")]
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
    File(PathBuf, Option<PathBuf>),
    Status(StatusCode),
}

#[derive(Debug, Error)]
pub enum BadRedirectTo {
    #[error("invalid uri: {0}")]
    InvalidUri(InvalidUri),
    #[error("invalid scheme: {0}")]
    InvalidScheme(String),
    #[error("invalid status code: {0}")]
    InvalidStatus(InvalidStatusCode),
    #[error("too many fallbacks provided")]
    TooManyFallbacks,
    #[error("fallback not allowed: {0}")]
    FallbackNotAllowed(String),
    #[error("path does not end with slash")]
    NoTrailingSlash,
    #[error("does not begin with scheme")]
    NoScheme,
}

impl FromStr for To {
    type Err = BadRedirectTo;
    fn from_str(to_str: &str) -> Result<Self, Self::Err> {
        let (path, fallback) = {
            let mut parts = to_str.split('|').fuse();
            match (parts.next(), parts.next(), parts.next()) {
                (Some(path), fallback, None) => (path, fallback),
                _ => return Err(BadRedirectTo::TooManyFallbacks),
            }
        };

        match path.parse::<Uri>() {
            Ok(uri) => match (uri.scheme().map(|s| s.as_str()), fallback) {
                (Some("http"), None) | (Some("https"), None) => {
                    let uri = uri.to_string();
                    match () {
                        _ if !uri.ends_with('/') => Err(BadRedirectTo::NoTrailingSlash),
                        _ => Ok(To::Http(uri)),
                    }
                }
                (Some("file"), fallback) => {
                    let uri = uri.authority().map_or("", |a| a.as_str()).to_string() + uri.path();
                    match () {
                        _ if !uri.ends_with('/') => Err(BadRedirectTo::NoTrailingSlash),
                        _ => Ok(To::File(PathBuf::from(uri), fallback.map(PathBuf::from))),
                    }
                }
                (Some("status"), None) => {
                    match StatusCode::from_bytes(
                        uri.authority().map_or("", |a| a.as_str()).as_bytes(),
                    ) {
                        Ok(status) => Ok(To::Status(status)),
                        Err(e) => Err(BadRedirectTo::InvalidStatus(e)),
                    }
                }
                (Some(scheme), None) => Err(BadRedirectTo::InvalidScheme(scheme.to_string())),
                (Some(_), Some(fallback)) => {
                    Err(BadRedirectTo::FallbackNotAllowed(fallback.to_string()))
                }
                (None, _) => Err(BadRedirectTo::NoScheme),
            },
            Err(e) => Err(BadRedirectTo::InvalidUri(e)),
        }
    }
}

#[derive(Debug, Error)]
pub enum BadRedirect {
    #[error("unequal number of `from` and `to` arguments")]
    UnequalFromTo,
}

#[derive(Debug)]
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
        self.redirects.iter().find_map(|(from, to)| {
            let req_path = match to {
                To::Http(..) => uri.path_and_query()?.as_str(),
                To::File(..) | To::Status(..) => uri.path(),
            };
            req_path.strip_prefix(from.0.as_str()).map(|req_tail| {
                Ok(match to {
                    To::Http(prefix) => Action::Http((prefix.to_string() + req_tail).parse()?),
                    To::File(prefix, fallback) => Action::File {
                        path: prefix.join(req_tail),
                        fallback: fallback.clone(),
                    },
                    To::Status(status) => Action::Status(*status),
                })
            })
        })
    }
}

pub enum Action {
    Http(Uri),
    File {
        path: PathBuf,
        fallback: Option<PathBuf>,
    },
    Status(StatusCode),
}

#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;

    case!(from_just_slash: assert!(matches!(From::from_str("/"), Ok(_))));
    case!(from_slash_api: assert!(matches!(From::from_str("/api/"), Ok(_))));
    case!(from_multi_slash: assert!(matches!(From::from_str("/resources/static/"), Ok(_))));

    case!(from_no_leading: assert!(matches!(From::from_str("foo/"), Err(BadRedirectFrom::NoLeadingSlash))));
    case!(from_no_trailing: assert!(matches!(From::from_str("/foo"), Err(BadRedirectFrom::NoTrailingSlash))));

    case!(to_localhost: assert!(matches!(To::from_str("http://localhost:3000/"), Ok(To::Http(_)))));
    case!(to_localhost_path: assert!(matches!(To::from_str("http://localhost:8080/services/api/"), Ok(To::Http(_)))));
    case!(to_localhost_https: assert!(matches!(To::from_str("https://localhost:8080/"), Ok(To::Http(_)))));
    case!(to_file: assert!(matches!(To::from_str("file://./"), Ok(To::File(_, None)))));
    case!(to_file_path: assert!(matches!(To::from_str("file://./static/"), Ok(To::File(_, None)))));
    case!(to_file_fallback: assert!(matches!(To::from_str("file://./static/|./static/index.html"), Ok(To::File(_, Some(_))))));

    case!(to_bad_uri: assert!(matches!(To::from_str("example.com/"), Err(BadRedirectTo::InvalidUri(_)))));
    case!(to_bad_scheme: assert!(matches!(To::from_str("ftp://example.com/"), Err(BadRedirectTo::InvalidScheme(_)))));
    case!(to_many_fallbacks: assert!(matches!(To::from_str("file://./|./|./"), Err(BadRedirectTo::TooManyFallbacks))));
    case!(to_bad_fallback: assert!(matches!(To::from_str("http://example.com/|./"), Err(BadRedirectTo::FallbackNotAllowed(_)))));
    case!(to_no_trailing: assert!(matches!(To::from_str("http://example.com/foo"), Err(BadRedirectTo::NoTrailingSlash))));
    case!(to_no_scheme: assert!(matches!(To::from_str("/foo"), Err(BadRedirectTo::NoScheme))));

    case!(rules_zip_unequal: assert!(matches!(Rules::zip(vec![From("/".to_string())], vec![]), Err(_))));
    case!(rules_zip: assert!(matches!(Rules::zip(vec![From("/".to_string())], vec![To::Http("/".to_string())]), Ok(_))));
}
