use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Status {
    /// The client is in the process of identifying itself.
    /// The server is waiting for the name and need to be validated.
    Identification,

    /// The client has successfully joined the chat.
    Joined,
}

struct Client {
    name: String,
    status: Status,
    tx: tokio::sync::broadcast::Sender<String>,
    rx: tokio::sync::broadcast::Receiver<String>,
}

pub async fn run(listener: tokio::net::TcpListener) -> anyhow::Result<()> {
    let (tx, rx) = tokio::sync::broadcast::channel(100);
    let participants = std::sync::Arc::new(std::sync::Mutex::new(vec![]));

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let c = Client {
                    name: String::new(),
                    status: Status::Identification,
                    tx: tx.clone(),
                    rx: rx.resubscribe(),
                };

                let p = participants.clone();

                tokio::spawn(async move {
                    match handler(stream, c, p).await {
                        Ok(_) => (),
                        Err(e) => tracing::error!("error on handling connection: {}", e),
                    }
                });
            }
            Err(e) => return Err(e.into()),
        }
    }
}

async fn handler(
    mut stream: tokio::net::TcpStream,
    mut client: Client,
    participants: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
) -> anyhow::Result<()> {
    // only first time the client will receive the welcome message
    let (mut r, mut w) = stream.split();
    let welcome_msg = b"Welcome to budgetchat! What shall I call you?\n";
    w.write_all(welcome_msg).await?;

    let mut bf = tokio::io::BufReader::new(&mut r);
    let mut buffer = Vec::new();

    loop {
        tokio::select! {
            n_bytes = bf.read_until(b'\n', &mut buffer) => {
                if n_bytes? == 0 {
                    break;
                }

                match client.status {
                        Status::Identification => {
                            buffer.pop(); // remove the newline

                            if !name_is_valid(&buffer) {
                                tracing::info!("name is invalid: {:?}", std::str::from_utf8(&buffer));
                                w.write_all(b"error: name is empty\n").await?;
                                break;
                            }

                            let name = String::from_utf8(buffer.clone()).unwrap();
                            client.status = Status::Joined;
                            client.name = name.to_string();
                            client.tx.send(format!("* {name} has entered the room")).unwrap();

                            let users_already_in = participants.lock().unwrap().join(", ");
                            w.write_all(format!("* The room contains: {}\n", users_already_in).as_bytes()).await?;

                            participants.lock().unwrap().push(name.to_string());
                        }

                        Status::Joined => _ = client.tx.send(format!("[{}] {}", client.name, String::from_utf8(buffer.clone()).unwrap())).unwrap(),
                }

                buffer.clear();
            }

            msg = client.rx.recv() => {
                if let Ok(msg) = msg {
                    if msg.contains(&client.name) {
                        continue;
                    }

                    let msg = if msg.ends_with('\n') { msg } else { format!("{}\n", msg) };

                    tracing::info!("sending the message: `{}` to {}", msg, client.name);
                    w.write_all(msg.as_bytes()).await?;
                }
            }
        }
    }

    if !client.name.is_empty() {
        client
            .tx
            .send(format!("* {} has left the room", client.name))
            .unwrap();

        participants.lock().unwrap().retain(|n| n != &client.name);
    }

    Ok(())
}

fn name_is_valid(name: &[u8]) -> bool {
    if name.is_empty() || name.len() > 32 {
        return false;
    }

    name.iter().all(|&c| c.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn join_user_ok() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("open a listener");

        let local_addr = listener.local_addr().expect("local address works");

        tokio::spawn(async move {
            run(listener).await.expect("run works");
        });

        // alice client
        let mut stream_alice = tokio::net::TcpStream::connect(local_addr)
            .await
            .expect("connection with local works");

        let (mut r_alice, mut w_alice) = stream_alice.split();
        w_alice.write_all(b"alice\n").await.expect("to write name");
        w_alice.flush().await.expect("to flush msg");

        // federico client
        let mut stream_federico = tokio::net::TcpStream::connect(local_addr)
            .await
            .expect("connection with local works");

        let (mut r_federico, mut w_federico) = stream_federico.split();
        let bf = tokio::io::BufReader::new(&mut r_federico);
        let mut s = bf.split(b'\n');
        let welcome_msg = s
            .next_segment()
            .await
            .expect("to read segment")
            .expect("to read segment");

        assert_eq!(
            welcome_msg,
            b"Welcome to budgetchat! What shall I call you?"
        );

        w_federico
            .write_all(b"federico\n")
            .await
            .expect("to write name");
        w_federico.flush().await.expect("to flush msg");

        w_federico
            .write_all(b"Just one more thing\n")
            .await
            .expect("to write name");

        let list_users = s
            .next_segment()
            .await
            .expect("to read segment")
            .expect("to read segment");

        assert_eq!(list_users, b"* The room contains: alice");

        let bf = tokio::io::BufReader::new(&mut r_alice);
        let mut s = bf.split(b'\n');
        let welcome_msg = s
            .next_segment()
            .await
            .expect("to read segment")
            .expect("to read segment");

        assert_eq!(
            welcome_msg,
            b"Welcome to budgetchat! What shall I call you?"
        );

        let room_empty = s
            .next_segment()
            .await
            .expect("to read segment")
            .expect("to read segment");

        assert_eq!(room_empty, b"* The room contains: ");

        let new_entry = s
            .next_segment()
            .await
            .expect("to read segment")
            .expect("to read segment");

        assert_eq!(new_entry, b"* federico has entered the room");

        let msg_from_federico = s
            .next_segment()
            .await
            .expect("to read segment")
            .expect("to read segment");

        assert_eq!(msg_from_federico, b"[federico] Just one more thing");

        w_federico.shutdown().await.expect("to shutdown");

        let leave_msg = s
            .next_segment()
            .await
            .expect("to read segment")
            .expect("to read segment");

        assert_eq!(leave_msg, b"* federico has left the room");
    }

    #[test]
    fn check_name() {
        assert!(!name_is_valid(b""));
        assert!(!name_is_valid(b" "));
        assert!(name_is_valid(b"John"));
        assert!(!name_is_valid(b"John Doe 123"));
        assert!(!name_is_valid(b"123456789012345678901234567890123"));
        assert!(name_is_valid(b"1234567890123456"));
        assert!(name_is_valid(b"LargeNewbie639"));
    }

    #[tokio::test]
    async fn illegal_name_for_join() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("open a listener");

        let local_addr = listener.local_addr().expect("local address works");

        tokio::spawn(async move {
            run(listener).await.expect("run works");
        });

        let mut stream = tokio::net::TcpStream::connect(local_addr)
            .await
            .expect("connection with local works");

        let (mut r, mut w) = stream.split();
        let mut bf = tokio::io::BufReader::new(&mut r);
        let mut v = vec![];
        bf.read_until(b'\n', &mut v)
            .await
            .expect("to read welcome msg");

        assert_eq!(v, b"Welcome to budgetchat! What shall I call you?\n");
        v.clear();

        w.write(b"\n").await.expect("to write name");
        w.flush().await.expect("to flush msg");
        w.shutdown().await.expect("shutdown");

        let mut s = bf.split(b'\n');

        let msg_error = s
            .next_segment()
            .await
            .expect("to read segment")
            .expect("to read segment");

        assert_eq!(msg_error, b"error: name is empty");
    }
}
