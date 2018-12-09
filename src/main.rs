mod err;
mod opt;
mod redir;
mod server;

use failure::Error;
use structopt::StructOpt;

#[global_allocator]
static ALLOC: std::alloc::System = std::alloc::System;

fn main() -> Result<(), err::DebugFromDisplay<Error>> {
    let opt::Options {
        verbose,
        from_port,
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

    server::run(&([0, 0, 0, 0], from_port).into(), redir::zip(from, to)?)?;

    Ok(())
}
