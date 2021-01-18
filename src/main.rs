#[macro_use]
mod macros;

mod err;
mod file;
mod opt;
mod redir;
mod routes;
mod server;

use structopt::StructOpt;

use crate::redir::Rules;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), err::DisplayError> {
    let opt::Options {
        verbose,
        listen_addr,
        from,
        to,
    } = opt::Options::from_args();

    env_logger::Builder::new()
        .filter_level(match verbose {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        })
        .init();

    server::run(&listen_addr, Rules::zip(from, to)?).await?;

    Ok(())
}
