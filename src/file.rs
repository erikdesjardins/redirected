use std::io;

use futures::{stream, try_ready};
use hyper::{Body, Chunk};
use tokio::fs::File;
use tokio::io::AsyncRead;

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
