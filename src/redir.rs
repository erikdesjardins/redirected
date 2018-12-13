use std::str::FromStr;

use failure::Fail;
use http::uri::InvalidUri;
use hyper::Uri;

use crate::util::IntoOptionExt;

#[derive(Debug)]
pub struct From(String);

#[derive(Debug, Fail)]
pub enum BadRedirectFrom {
    #[fail(display = "path does not start with '/'")]
    MissingSlash,
    #[fail(display = "path does not end with '*'")]
    MissingWildcard,
}

impl FromStr for From {
    type Err = BadRedirectFrom;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('/') {
            let mut path = s.to_string();
            match path.pop() {
                Some('*') => Ok(From(path)),
                _ => Err(BadRedirectFrom::MissingWildcard),
            }
        } else {
            Err(BadRedirectFrom::MissingSlash)
        }
    }
}

#[derive(Debug)]
pub enum To {
    Http(String),
}

#[derive(Debug, Fail)]
pub enum BadRedirectTo {
    #[fail(display = "invalid uri: {}", _0)]
    InvalidUri(InvalidUri),
    #[fail(display = "uri does not end with '*'")]
    MissingWildcard,
}

impl FromStr for To {
    type Err = BadRedirectTo;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<Uri>() {
            Ok(uri) => {
                let mut uri = uri.to_string();
                match uri.pop() {
                    Some('*') => Ok(To::Http(uri)),
                    _ => Err(BadRedirectTo::MissingWildcard),
                }
            }
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
                    })
                })
        })
    }
}

pub enum Action {
    Http(Uri),
}
