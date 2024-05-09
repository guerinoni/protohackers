use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await?;

    loop {
        let stream = match listener.accept().await {
            Ok((stream, address)) => {
                println!("connection received for {}", address);

                stream
            }
            Err(e) => return Err(e.into()),
        };

        tokio::spawn(handler(stream));
    }
}

async fn handler(mut stream: tokio::net::TcpStream) -> anyhow::Result<()> {
    loop {
        let mut buffer = [0; 1024];
        match stream.read(&mut buffer).await? {
            0 => break,
            bytes_read => _ = stream.write(&buffer[..bytes_read]).await?,
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn echo() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("open a listener");

        let local_addr = listener.local_addr().expect("local address works");

        tokio::spawn(async move {
            loop {
                let stream = match listener.accept().await {
                    Ok((stream, address)) => {
                        println!("connection received for {}", address);

                        stream
                    }
                    Err(_) => panic!("error on accepting connection"),
                };

                tokio::spawn(handler(stream));
            }
        });

        let mut stream = tokio::net::TcpStream::connect(local_addr)
            .await
            .expect("connection with local works");

        let (mut r, mut w) = stream.split();

        let written = w
            .write(b"hello echo server\n")
            .await
            .expect("to write payload");
        w.flush().await.expect("to flush msg");
        w.shutdown().await.expect("shutdown");

        assert_ne!(written, 0);

        let mut buffer = [0; 1024];
        let mut last_char = 0;
        loop {
            let bytes_read = r.read(&mut buffer).await.expect("read msg");
            if bytes_read == 0 {
                break;
            }

            last_char += bytes_read;
        }

        assert_eq!(&buffer[..last_char], b"hello echo server\n");
    }
}
