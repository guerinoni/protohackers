use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

pub async fn run(
    listener: tokio::net::TcpListener,
    chat_address: &str,
    boguscoin: &str,
) -> anyhow::Result<()> {
    loop {
        let (stream, address) = listener.accept().await?;

        tracing::info!("connection received for {}", address);

        let chat_address = chat_address.to_string();
        let boguscoin = boguscoin.to_string();

        tokio::spawn(async move {
            match handle(stream, &chat_address, &boguscoin).await {
                Ok(_) => (),
                Err(e) => {
                    tracing::error!("error handling connection from {}: {}", address, e);
                }
            }
        });
    }
}

async fn handle(
    stream_proxy: tokio::net::TcpStream,
    chat_address: &str,
    boguscoin: &str,
) -> anyhow::Result<()> {
    let (mut proxy_reader, mut proxy_writer) = {
        let (r, w) = stream_proxy.into_split();
        (tokio::io::BufReader::new(r), w)
    };

    let (mut chat_reader, mut chat_writer) = {
        let (r, w) = tokio::net::TcpStream::connect(chat_address)
            .await?
            .into_split();
        (tokio::io::BufReader::new(r), w)
    };

    let boguscoin = boguscoin.to_string();
    let bog = boguscoin.to_string();

    let chat_to_proxy = tokio::spawn(async move {
        loop {
            let mut line = String::new();
            match chat_reader.read_line(&mut line).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    line.pop();
                    let resp = replace_message(line, &boguscoin);
                    tracing::info!("chat -> proxy: {}", resp);
                    proxy_writer.write_all(resp.as_bytes()).await.unwrap();
                    proxy_writer.write_u8(b'\n').await.unwrap();
                }
            }
        }
    });

    let proxy_to_chat = tokio::spawn(async move {
        loop {
            let mut line = String::new();
            match proxy_reader.read_line(&mut line).await {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    line.pop();
                    let resp = replace_message(line, &bog);
                    tracing::info!("proxy -> chat: {}", resp);
                    chat_writer.write_all(resp.as_bytes()).await.unwrap();
                    chat_writer.write_u8(b'\n').await.unwrap();
                }
            }
        }
    });

    tokio::select! {
        _ = chat_to_proxy => {},
        _ = proxy_to_chat => {},
    }

    Ok(())
}

fn replace_message(msg: String, boguscoin: &str) -> String {
    let parts: Vec<String> = msg
        .split(' ')
        .map(|s| {
            if s.starts_with('7')
                && 26 <= s.len()
                && s.len() <= 35
                && s.chars().all(char::is_alphanumeric)
            {
                boguscoin
            } else {
                s
            }
            .to_string()
        })
        .collect();

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn check() {
        let subscriber = tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(tracing::Level::INFO)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");

        let budget_chat_port = {
            // start the budget chat server
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("open a listener");

            let local_addr = listener.local_addr().expect("local address works");

            tokio::spawn(async move {
                crate::budget_chat::run(listener).await.expect("run works");
            });

            tracing::info!("budget chat server started at {}", local_addr);

            local_addr.port()
        };

        // start the mob in the middle server (proxy)
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("open a listener");

        let addr = listener.local_addr().expect("local address works");
        let port = addr.port();

        tokio::spawn(async move {
            run(
                listener,
                &format!("127.0.0.1:{budget_chat_port}"),
                "7YWHMfk9JZe0LM0g1ZauHuiSxhI",
            )
            .await
            .expect("run works");
        });

        tracing::info!("proxy started at {}", addr);

        // alice connect to chat
        let mut s_alice = tokio::net::TcpStream::connect(format!("127.0.0.1:{budget_chat_port}"))
            .await
            .expect("connection with local works");
        let (r_alice, mut w_alice) = s_alice.split();
        let mut buf_r_alice = tokio::io::BufReader::new(r_alice).split(b'\n');
        w_alice.write_all(b"alice\n").await.expect("to write name");

        let welcome = buf_r_alice.next_segment().await.unwrap().unwrap();
        assert_eq!(welcome, b"Welcome to budgetchat! What shall I call you?");

        let contains = buf_r_alice.next_segment().await.unwrap().unwrap();
        assert_eq!(contains, b"* The room contains: ");

        // federico connect to proxy
        let mut s_federico = tokio::net::TcpStream::connect(format!("127.0.0.1:{port}"))
            .await
            .expect("connection with local works");
        let (r_federico, mut w_federico) = s_federico.split();
        let mut buf_r_federico = tokio::io::BufReader::new(r_federico).split(b'\n');

        let welcome_msg = buf_r_federico.next_segment().await.unwrap().unwrap();
        assert_eq!(
            welcome_msg,
            b"Welcome to budgetchat! What shall I call you?"
        );

        w_federico
            .write_all(b"federico\n")
            .await
            .expect("to write name");

        let result = buf_r_federico.next_segment().await.unwrap().unwrap();
        assert_eq!(result, b"* The room contains: alice");

        let entered = buf_r_alice.next_segment().await.unwrap().unwrap();
        assert_eq!(entered, b"* federico has entered the room");

        w_federico
            .write_all(b"Hi alice, please pay 100$ to 7iKDZEwPZSqIvDnHvVN2r0hUWXD5rHX\n")
            .await
            .unwrap();

        let result = buf_r_alice.next_segment().await.unwrap().unwrap();

        assert_eq!(
            result,
            b"[federico] Hi alice, please pay 100$ to 7YWHMfk9JZe0LM0g1ZauHuiSxhI"
        );
    }
}
