use std::path::PathBuf;
use std::str::FromStr;

use http::status::InvalidStatusCode;
use http::uri::{InvalidUri, Scheme};
use hyper::{StatusCode, Uri};
use thiserror::Error;

use crate::util::IntoOptionExt;

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
            Ok(uri) => match (uri.scheme_part().map(Scheme::as_str), fallback) {
                (Some("http"), None) | (Some("https"), None) => {
                    let uri = uri.to_string();
                    match () {
                        _ if !uri.ends_with('/') => Err(BadRedirectTo::NoTrailingSlash),
                        _ => Ok(To::Http(uri)),
                    }
                }
                (Some("file"), fallback) => {
                    let uri =
                        uri.authority_part().map_or("", |a| a.as_str()).to_string() + uri.path();
                    match () {
                        _ if !uri.ends_with('/') => Err(BadRedirectTo::NoTrailingSlash),
                        _ => Ok(To::File(PathBuf::from(uri), fallback.map(PathBuf::from))),
                    }
                }
                (Some("status"), None) => {
                    match StatusCode::from_bytes(
                        uri.authority_part().map_or("", |a| a.as_str()).as_bytes(),
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
            req_path
                .trim_start_matches(from.0.as_str())
                .some_if(|&t| t != req_path)
                .map(|req_tail| {
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

    case!(from_just_slash: assert_matches!(Ok(_), From::from_str("/")));
    case!(from_slash_api: assert_matches!(Ok(_), From::from_str("/api/")));
    case!(from_multi_slash: assert_matches!(Ok(_), From::from_str("/resources/static/")));

    case!(from_no_leading: assert_matches!(Err(BadRedirectFrom::NoLeadingSlash), From::from_str("foo/")));
    case!(from_no_trailing: assert_matches!(Err(BadRedirectFrom::NoTrailingSlash), From::from_str("/foo")));

    case!(to_localhost: assert_matches!(Ok(To::Http(_)), To::from_str("http://localhost:3000/")));
    case!(to_localhost_path: assert_matches!(Ok(To::Http(_)), To::from_str("http://localhost:8080/services/api/")));
    case!(to_localhost_https: assert_matches!(Ok(To::Http(_)), To::from_str("https://localhost:8080/")));
    case!(to_file: assert_matches!(Ok(To::File(_, None)), To::from_str("file://./")));
    case!(to_file_path: assert_matches!(Ok(To::File(_, None)), To::from_str("file://./static/")));
    case!(to_file_fallback: assert_matches!(Ok(To::File(_, Some(_))), To::from_str("file://./static/|./static/index.html")));

    case!(to_bad_uri: assert_matches!(Err(BadRedirectTo::InvalidUri(_)), To::from_str("example.com/")));
    case!(to_bad_scheme: assert_matches!(Err(BadRedirectTo::InvalidScheme(_)), To::from_str("ftp://example.com/")));
    case!(to_many_fallbacks: assert_matches!(Err(BadRedirectTo::TooManyFallbacks), To::from_str("file://./|./|./")));
    case!(to_bad_fallback: assert_matches!(Err(BadRedirectTo::FallbackNotAllowed(_)), To::from_str("http://example.com/|./")));
    case!(to_no_trailing: assert_matches!(Err(BadRedirectTo::NoTrailingSlash), To::from_str("http://example.com/foo")));
    case!(to_no_scheme: assert_matches!(Err(BadRedirectTo::NoScheme), To::from_str("/foo")));

    case!(rules_zip_unequal: assert_matches!(Err(_), Rules::zip(vec![From("/".to_string())], vec![])));
    case!(rules_zip: assert_matches!(Ok(_), Rules::zip(vec![From("/".to_string())], vec![To::Http("/".to_string())])));
}
