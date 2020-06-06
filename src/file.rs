use futures::stream;
use hyper::Body;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub fn body_stream(file: File) -> Body {
    Body::wrap_stream(stream::try_unfold(file, {
        let mut buf = [0; 4 * 1024];
        move |mut file| async move {
            match file.read(&mut buf).await {
                Ok(0) => Ok(None),
                Ok(n) => Ok(Some((buf[..n].to_vec(), file))),
                Err(e) => Err(e),
            }
        }
    }))
}
