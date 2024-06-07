pub async fn run() -> anyhow::Result<()> {
    let listener = tokio::net::UdpSocket::bind("0.0.0.0:8000").await?;
    tracing::info!("Listening on {}", listener.local_addr()?);

    let mut db = std::collections::HashMap::new();

    let mut buffer = vec![0; 1024];
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("Received Ctrl-C");
                break;
            }

            v = listener.recv_from(&mut buffer) => {
                match v {
                    Ok((size, addr)) => {
                        tracing::info!("Received {} bytes from {}", size, addr);

                        let buffer = &buffer[..size];
                        tracing::info!(
                            "Received {} bytes from {} -> {:?}",
                            size,
                            addr,
                            std::str::from_utf8(buffer)
                        );

                        if let Some(response) = do_it(buffer, &mut db)? {
                            listener.send_to(response.as_bytes(), addr).await?;
                        }
                    }

                    Err(e) => {
                        tracing::error!("Error receiving from UDP socket: {}", e);
                    }
                }
            }
        }
    }

    Ok(())
}

fn do_it(
    buffer: &[u8],
    db: &mut std::collections::HashMap<String, String>,
) -> anyhow::Result<Option<String>> {
    let equal_sign = b'=';

    let ret = if buffer.contains(&equal_sign) {
        let (key, value) = buffer.split_at(buffer.iter().position(|&x| x == b'=').unwrap());
        if key == b"version" {
            None
        } else {
            let value = String::from_utf8_lossy(&value[1..]).to_string();
            let key = String::from_utf8_lossy(key).to_string();
            db.insert(key, value);
            None
        }
    } else {
        let key = String::from_utf8_lossy(buffer).to_string();

        let v = if &key == "version" {
            "version=1.0.0".to_string()
        } else {
            let value = db.get(&key).map(|x| x.as_str()).unwrap_or("");
            let mut result = key.clone();
            result.push('=');
            result.push_str(value);
            result
        };

        Some(v)
    };

    Ok(ret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieve() {
        {
            let mut db = std::collections::HashMap::new();

            // insert a value
            let buffer = b"foo=bar";
            assert!(do_it(buffer, &mut db).is_ok());

            let buffer = b"foo";
            let result = do_it(buffer, &mut db);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("foo=bar".to_string()));

            let buffer = b"version";
            let result = do_it(buffer, &mut db);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("version=1.0.0".to_string()));
        }
        {
            let mut db = std::collections::HashMap::new();

            // search for a non-existing value
            let buffer = b"aaa";
            let result = do_it(buffer, &mut db);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), Some("aaa=".to_string()));
        }
    }

    #[test]
    fn insert() {
        {
            let mut db = std::collections::HashMap::new();
            let buffer = b"foo=bar";
            assert!(do_it(buffer, &mut db).is_ok());
            assert_eq!(db.get("foo"), Some(&"bar".to_string()));
        }
        {
            let mut db = std::collections::HashMap::new();
            let buffer = b"foo=bar=f";
            assert!(do_it(buffer, &mut db).is_ok());
            assert_eq!(db.get("foo"), Some(&"bar=f".to_string()));
        }
        {
            let mut db = std::collections::HashMap::new();
            let buffer = b"foo===";
            assert!(do_it(buffer, &mut db).is_ok());
            assert_eq!(db.get("foo"), Some(&"==".to_string()));
        }
        {
            let mut db = std::collections::HashMap::new();
            let buffer = b"foo=";
            assert!(do_it(buffer, &mut db).is_ok());
            assert_eq!(db.get("foo"), Some(&"".to_string()));
        }
        {
            let mut db = std::collections::HashMap::new();
            let buffer = b"=bar";
            assert!(do_it(buffer, &mut db).is_ok());
            assert_eq!(db.get(""), Some(&"bar".to_string()));
        }
        {
            let mut db = std::collections::HashMap::new();
            let buffer = b"version=111";
            assert!(do_it(buffer, &mut db).is_ok());
            assert_eq!(db.get("version"), None); // never store version
        }
    }

    #[test]
    fn update() {
        let mut db = std::collections::HashMap::new();
        let buffer = b"foo=bar";
        assert!(do_it(buffer, &mut db).is_ok());
        assert_eq!(db.get("foo"), Some(&"bar".to_string()));

        let buffer = b"foo=bar2";
        assert!(do_it(buffer, &mut db).is_ok());
        assert_eq!(db.get("foo"), Some(&"bar2".to_string()));
    }
}
