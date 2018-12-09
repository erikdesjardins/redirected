use std::ops::Deref;
use std::str::FromStr;

use failure::{Error, Fail};
use http::uri::InvalidUri;
use hyper::Uri;

#[derive(Debug)]
pub struct RedirectPath(String);

impl Deref for RedirectPath {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

#[derive(Debug, Fail)]
pub enum BadRedirectPath {
    #[fail(display = "path does not start with '/'")]
    MissingSlash,
    #[fail(display = "path does not end with '*'")]
    MissingWildcard,
}

impl FromStr for RedirectPath {
    type Err = BadRedirectPath;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with('/') {
            let mut path = s.to_string();
            match path.pop() {
                Some('*') => Ok(RedirectPath(path)),
                _ => Err(BadRedirectPath::MissingWildcard),
            }
        } else {
            Err(BadRedirectPath::MissingSlash)
        }
    }
}

#[derive(Debug)]
pub struct RedirectUri(String);

impl Deref for RedirectUri {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

#[derive(Debug, Fail)]
pub enum BadRedirectUri {
    #[fail(display = "invalid uri: {}", _0)]
    InvalidUri(InvalidUri),
    #[fail(display = "uri does not end with '*'")]
    MissingWildcard,
}

impl FromStr for RedirectUri {
    type Err = BadRedirectUri;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.parse::<Uri>() {
            Ok(uri) => {
                let mut uri = uri.to_string();
                match uri.pop() {
                    Some('*') => Ok(RedirectUri(uri)),
                    _ => Err(BadRedirectUri::MissingWildcard),
                }
            }
            Err(e) => Err(BadRedirectUri::InvalidUri(e)),
        }
    }
}

#[derive(Debug)]
pub struct Redirect {
    pub from: RedirectPath,
    pub to: RedirectUri,
}

#[derive(Debug, Fail)]
pub enum BadRedirect {
    #[fail(display = "unequal number of `from` and `to` arguments")]
    UnequalFromTo,
}

pub fn zip(from: Vec<RedirectPath>, to: Vec<RedirectUri>) -> Result<Vec<Redirect>, Error> {
    if from.len() == to.len() {
        Ok(from
            .into_iter()
            .zip(to)
            .map(|(from, to)| Redirect { from, to })
            .collect())
    } else {
        Err(BadRedirect::UnequalFromTo.into())
    }
}
