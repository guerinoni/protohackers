mod means_to_an_end;
mod prime_time;
mod smoke_test;

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

        tokio::spawn(async {
            match means_to_an_end::handler(stream).await {
                Ok(_) => (),
                Err(e) => eprintln!("error: {:?}", e),
            }
        });
    }
}
