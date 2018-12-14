use structopt::StructOpt;

use crate::redir::{From, To};

#[derive(StructOpt, Debug)]
pub struct Options {
    /// Logging verbosity (-v info, -vv debug, -vvv trace)
    #[structopt(
        short = "v",
        long = "verbose",
        parse(from_occurrences),
        raw(global = "true")
    )]
    pub verbose: u8,

    /// Port to redirect from, e.g. `8080`
    pub from_port: u16,

    /// Paths to redirect from, e.g. `/api/*`, `/resources/static/*`
    #[structopt(
        short = "f",
        long = "from",
        raw(required = "true"),
        parse(try_from_str)
    )]
    pub from: Vec<From>,

    /// Addresses to redirect to, e.g. `http://localhost:3000/*`, `file://./static/*`
    #[structopt(short = "t", long = "to", raw(required = "true"), parse(try_from_str))]
    pub to: Vec<To>,
}
