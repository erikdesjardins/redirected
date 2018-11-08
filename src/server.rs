use std::collections::HashMap;
use std::net::SocketAddr;

use failure::Error;

use redir::Redir;

pub fn run(mappings: HashMap<SocketAddr, Vec<Redir>>) -> Result<(), Error> {
    Ok(())
}
