use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub async fn handler(mut stream: tokio::net::TcpStream) -> anyhow::Result<()> {
    let (r, mut w) = stream.split();
    let mut bf = tokio::io::BufReader::new(r);

    let mut db_memory = std::collections::BTreeMap::new();

    loop {
        let mut action = [0u8; 9];
        bf.read_exact(&mut action).await?;

        match action[0] {
            b'I' => {
                let t: [u8; 4] = action[1..=4].try_into()?;
                let timestamp = i32::from_be_bytes(t);

                let p: [u8; 4] = action[5..=8].try_into()?;
                let price = i32::from_be_bytes(p);

                db_memory.insert(timestamp, price);
            }
            b'Q' => {
                let l: [u8; 4] = action[1..=4].try_into()?;
                let left = i32::from_be_bytes(l);
                let r: [u8; 4] = action[5..=8].try_into()?;
                let right = i32::from_be_bytes(r);

                if left > right {
                    println!("invalid range");
                    w.write_i32(0).await?;
                    continue;
                }

                let it = db_memory.range((
                    std::ops::Bound::Included(&left),
                    std::ops::Bound::Included(&right),
                ));

                let mut mean: i64 = 0;
                let mut counter = 0;
                it.for_each(|(_, &x)| {
                    mean += x as i64;
                    counter += 1;
                });

                let mean = if counter == 0 { mean } else { mean / counter };
                w.write_i32(mean.try_into()?).await?;
            }
            _ => break,
        }
    }

    w.flush().await?;
    w.shutdown().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn range_left_greather_right() {
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

        w.write_u8(b'I').await.unwrap();
        w.write_i32(12345).await.unwrap();
        w.write_i32(101).await.unwrap();
        w.flush().await.expect("to flush msg");

        w.write_u8(b'Q').await.unwrap();
        w.write_i32(20000).await.unwrap();
        w.write_i32(16384).await.unwrap();
        w.flush().await.expect("to flush msg");

        w.shutdown().await.expect("shutdown");

        let resp = r.read_i32().await.expect("reading response");
        assert_eq!(resp, 0);
    }

    #[tokio::test]
    async fn example_ok() {
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

        w.write_u8(b'I').await.unwrap();
        w.write_i32(12345).await.unwrap();
        w.write_i32(101).await.unwrap();
        w.flush().await.expect("to flush msg");

        w.write_u8(b'Q').await.unwrap();
        w.write_i32(12288).await.unwrap();
        w.write_i32(16384).await.unwrap();
        w.flush().await.expect("to flush msg");

        w.shutdown().await.expect("shutdown");

        let resp = r.read_i32().await.expect("reading response");
        assert_eq!(resp, 101);
    }
}
