use std::io;
use std::path::PathBuf;

use futures::future::Either::{A, B};
use futures::{future, stream, try_ready, Future};
use hyper::{Body, Chunk};
use tokio::fs::File;
use tokio::io::AsyncRead;

pub fn resolve_or_index(path: PathBuf) -> impl Future<Item = File, Error = io::Error> {
    let index_path = path.join("index.html");
    File::open(path).or_else(move |e| {
        if let io::ErrorKind::PermissionDenied = e.kind() {
            A(File::open(index_path))
        } else {
            B(future::err(e))
        }
    })
}

pub fn body_stream(mut file: File) -> Body {
    Body::wrap_stream(stream::poll_fn({
        let mut buf = [0; 4 * 1024];
        move || -> Result<_, io::Error> {
            match try_ready!(file.poll_read(&mut buf)) {
                0 => Ok(None.into()),
                n => Ok(Some(Chunk::from(buf[..n].to_owned())).into()),
            }
        }
    }))
}
