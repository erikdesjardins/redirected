use hyper::Uri;
use structopt::StructOpt;

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

    /// Address to redirect from, e.g. `http://localhost:3000/api/*`
    #[structopt(short = "f", long = "from")]
    pub from: Vec<Uri>,

    /// Address to redirect to, e.g. `http://localhost:8080/*`
    #[structopt(short = "t", long = "to")]
    pub to: Vec<Uri>,
}
