use structopt::StructOpt;

use crate::redir::{From, To};

#[derive(StructOpt, Debug)]
pub struct Options {
    #[structopt(
        short = "v",
        long = "verbose",
        parse(from_occurrences),
        raw(global = "true"),
        help = "Logging verbosity (-v info, -vv debug, -vvv trace)"
    )]
    pub verbose: u8,

    #[structopt(
        help = "Port to redirect from (--help for more)",
        long_help = r"Port to redirect from

Behavior:
    - incoming http connections are received on this port

Examples:
    - 3000"
    )]
    pub from_port: u16,

    #[structopt(
        short = "f",
        long = "from",
        raw(required = "true"),
        parse(try_from_str),
        help = "Path prefixes to redirect from (--help for more)",
        long_help = r"Path prefixes to redirect from

Behavior:
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
        raw(required = "true"),
        parse(try_from_str),
        help = "Address prefixes to redirect to (--help for more)",
        long_help = r"Address prefixes to redirect to

Behavior:
    - each matching request's tail is appended to the corresponding address prefix
    - some schemes have special behavior

Examples:
    - http://localhost:8080/services/api/
    - file://./static/
    - file://./static/|./static/index.html (fallback to ./static/index.html)
    - status://404 (empty response with status 404)"
    )]
    pub to: Vec<To>,
}
