use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

#[derive(serde::Deserialize, Debug)]
struct IsPrimeRequest {
    method: String,
    number: serde_json::Number,
}

#[derive(serde::Serialize)]
struct IsPrimeResponse {
    method: String,
    prime: bool,
}

pub async fn handler(mut stream: tokio::net::TcpStream) -> anyhow::Result<()> {
    let (r, mut w) = stream.split();
    let mut bf = tokio::io::BufReader::new(r);
    let mut buffer = vec![];

    loop {
        buffer.clear();
        match bf.read_until(b'\n', &mut buffer).await? {
            0 => break,
            bytes_read => match validate(&buffer[..bytes_read]) {
                Ok(v) => {
                    let resp = IsPrimeResponse {
                        method: "isPrime".to_string(),
                        prime: v,
                    };
                    let body = serde_json::to_vec(&resp)?;
                    w.write_all(&body).await?;
                    w.write_u8(b'\n').await?;
                }
                Err(e) => w.write_all(e.as_bytes()).await?,
            },
        }
    }

    w.flush().await?;
    w.shutdown().await?;

    Ok(())
}

fn validate(buffer: &[u8]) -> Result<bool, &'static str> {
    let req = serde_json::from_slice::<IsPrimeRequest>(buffer).map_err(|_| "error")?;

    if req.method != "isPrime" {
        return Err("error");
    }

    match req.number.as_u64() {
        Some(n) => Ok(is_prime(n)),
        None => Ok(false),
    }
}

fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }

    for i in 2..n {
        if n % i == 0 {
            return false;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    use tokio::io::AsyncReadExt;

    #[test]
    fn check_prime() {
        assert!(is_prime(11));
        assert!(!is_prime(10));
    }

    #[tokio::test]
    async fn number_not_prime() {
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

        let (r, mut w) = stream.split();

        let payload = br#"{"method":"isPrime","number":123}"#;
        let written = w.write(payload).await.expect("to write payload");
        w.write_u8(b'\n').await.unwrap();

        w.flush().await.expect("to flush msg");
        w.shutdown().await.expect("shutdown");

        assert_ne!(written, 0);

        let mut buffer = vec![];
        let mut bf = tokio::io::BufReader::new(r);
        let n = bf.read_until(b'\n', &mut buffer).await.unwrap();
        assert!(n > 0);

        let s = std::str::from_utf8(&buffer).unwrap();
        assert_eq!(s, "{\"method\":\"isPrime\",\"prime\":false}\n");
    }

    #[tokio::test]
    async fn number_prime() {
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

        let (r, mut w) = stream.split();

        let payload = br#"{"method":"isPrime","number":11}"#;
        let written = w.write(payload).await.expect("to write payload");

        w.flush().await.expect("to flush msg");
        w.shutdown().await.expect("shutdown");

        assert_ne!(written, 0);

        let mut buffer = vec![];
        let mut bf = tokio::io::BufReader::new(r);
        let n = bf.read_until(b'\n', &mut buffer).await.unwrap();
        assert!(n > 0);

        let s = std::str::from_utf8(&buffer).unwrap();
        assert_eq!(s, "{\"method\":\"isPrime\",\"prime\":true}\n");
    }

    #[tokio::test]
    async fn different_method() {
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

        let payload = br#"{"method":"aaaa","number":11}"#;
        let written = w.write(payload).await.expect("to write payload");

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

        let resp = String::from_utf8(buffer[..last_char].to_vec()).unwrap();
        let expected = r#"error"#;
        assert_eq!(resp, expected);
    }
}
