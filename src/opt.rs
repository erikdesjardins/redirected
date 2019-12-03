use std::net::SocketAddr;

use structopt::StructOpt;

use crate::redir::{From, To};

#[derive(StructOpt, Debug)]
#[structopt(about)]
pub struct Options {
    #[structopt(
        short = "v",
        long = "verbose",
        parse(from_occurrences),
        global = true,
        help = "Logging verbosity (-v info, -vv debug, -vvv trace)"
    )]
    pub verbose: u8,

    #[structopt(
        help = "Socket address to listen on (--help for more)",
        long_help = r"Socket address to listen on:
    - incoming http connections are received on this socket
Examples:
    - 127.0.0.1:3000
    - 0.0.0.0:80
    - [2001:db8::1]:8080"
    )]
    pub listen_addr: SocketAddr,

    #[structopt(
        short = "f",
        long = "from",
        required = true,
        parse(try_from_str),
        help = "Path prefixes to redirect from (--help for more)",
        long_help = r"Path prefixes to redirect from:
    - each prefix is checked in order, and the first match is chosen
    - 404s if no prefixes match
Examples:
    - /
    - /resources/static/"
    )]
    pub from: Vec<From>,

    #[structopt(
        short = "t",
        long = "to",
        required = true,
        parse(try_from_str),
        help = "Address prefixes to redirect to (--help for more)",
        long_help = r"Address prefixes to redirect to:
    - each matching request's tail is appended to the corresponding address prefix
    - some schemes have special behavior
Examples:
    - http://localhost:8080/services/api/
    - https://test.dev/v1/
    - file://./static/
    - file://./static/|./static/index.html (fallback to ./static/index.html)
    - status://404 (empty response with status 404)"
    )]
    pub to: Vec<To>,
}
