use std::collections::HashMap;
use std::net::{SocketAddr, ToSocketAddrs};

use failure::{Error, Fail, ResultExt};
use hyper::Uri;

use util::OptionExt;

#[derive(Debug, PartialEq)]
pub struct Redir {
    pub from: String,
    pub to: String,
}

pub fn parse(from: Vec<Uri>, to: Vec<Uri>) -> Result<HashMap<SocketAddr, Vec<Redir>>, Error> {
    #[derive(Debug, Fail)]
    #[fail(display = "Unequal number of `from` and `to` addresses")]
    struct UnequalFromTo;

    #[derive(Debug, Fail)]
    #[fail(display = "{} -> {}: {}", _0, _1, _2)]
    struct ParseError(String, String, Error);

    #[derive(Debug, Fail)]
    #[fail(display = "Expected address to end with '*'")]
    struct MustEndWithWildcard;

    if from.len() != to.len() {
        return Err(UnequalFromTo.into());
    }

    let mut mappings = HashMap::<_, Vec<_>>::new();

    for (from, to) in from.into_iter().zip(to) {
        let go = || -> Result<_, Error> {
            let from_addr = best_socket_addr(
                from.authority_part()
                    .into_result()
                    .context("missing host")?
                    .as_str(),
            )?;

            let mut from_path = from
                .path_and_query()
                .into_result()
                .context("missing path")?
                .as_str()
                .to_string();

            let mut to_full = to
                .scheme_part()
                .into_result()
                .context("missing protocol")?
                .as_str()
                .to_string()
                + "://"
                + to.authority_part()
                    .into_result()
                    .context("missing host")?
                    .as_str()
                + to.path_and_query()
                    .into_result()
                    .context("missing path")?
                    .as_str();

            let rule = match (from_path.pop(), to_full.pop()) {
                (Some('*'), Some('*')) => Redir {
                    from: from_path,
                    to: to_full,
                },
                _ => return Err(MustEndWithWildcard.into()),
            };

            Ok((from_addr, rule))
        };
        match go() {
            Ok((addr, rule)) => mappings.entry(addr).or_default().push(rule),
            Err(e) => return Err(ParseError(from.to_string(), to.to_string(), e).into()),
        }
    }

    Ok(mappings)
}

fn best_socket_addr(host: &str) -> Result<SocketAddr, Error> {
    Ok(host
        .to_socket_addrs()?
        .find(SocketAddr::is_ipv4)
        .into_result()
        .context("no matching hosts found")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unequal() {
        let res = parse(vec![], vec!["localhost:80".parse().unwrap()]);
        assert_eq!(
            res.err().unwrap().to_string(),
            "Unequal number of `from` and `to` addresses"
        );
    }

    #[test]
    fn missing_host() {
        let res = parse(
            vec!["/test".parse().unwrap()],
            vec!["http://localhost:80".parse().unwrap()],
        );
        assert_eq!(
            res.err().unwrap().to_string(),
            "/test -> http://localhost:80/: missing host"
        );
    }

    #[test]
    fn invalid_addr() {
        let res = parse(
            vec!["http://example.com".parse().unwrap()],
            vec!["http://localhost:80".parse().unwrap()],
        );
        assert_eq!(
            res.err().unwrap().to_string(),
            "http://example.com/ -> http://localhost:80/: invalid socket address"
        );
    }

    #[test]
    fn missing_path() {
        let res = parse(
            vec!["localhost:3000".parse().unwrap()],
            vec!["http://localhost:80".parse().unwrap()],
        );
        assert_eq!(
            res.err().unwrap().to_string(),
            "localhost:3000 -> http://localhost:80/: missing path"
        );
    }

    #[test]
    fn missing_protocol() {
        let res = parse(
            vec!["http://localhost:3000".parse().unwrap()],
            vec!["localhost:80".parse().unwrap()],
        );
        assert_eq!(
            res.err().unwrap().to_string(),
            "http://localhost:3000/ -> localhost:80: missing protocol"
        );
    }

    #[test]
    fn works() {
        let res = parse(
            vec!["http://localhost:3000/api/*".parse().unwrap()],
            vec!["http://localhost:8080/*".parse().unwrap()],
        );
        assert_eq!(
            res.unwrap(),
            vec![(
                SocketAddr::from(([127, 0, 0, 1], 3000)),
                vec![Redir {
                    from: "/api/".to_string(),
                    to: "http://localhost:8080/".to_string(),
                }]
            )].into_iter()
            .collect()
        )
    }
}
