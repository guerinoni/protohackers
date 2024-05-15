mod means_to_an_end;
mod prime_time;
mod smoke_test;
use std::io::BufRead;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await?;

    let input = std::io::stdin();
    println!("select which exercise to run: ");
    println!("0. Smoke Test");
    println!("1. Prime Time");
    println!("2. Means to an End");

    let selected_exercise = input.lock().lines().next().unwrap()?;
    let handler = chooser(selected_exercise.parse()?);

    loop {
        let stream = match listener.accept().await {
            Ok((stream, address)) => {
                tracing::info!("connection received for {}", address);

                stream
            }
            Err(e) => return Err(e.into()),
        };

        tokio::spawn(async move {
            match handler(stream).await {
                Ok(_) => (),
                Err(e) => tracing::error!("error on handling connection: {}", e),
            }
        });
    }
}

fn chooser(
    input: u32,
) -> fn(
    tokio::net::TcpStream,
) -> std::pin::Pin<
    std::boxed::Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + 'static>,
> {
    match input {
        0 => |s| Box::pin(smoke_test::handler(s)),
        1 => |s| Box::pin(prime_time::handler(s)),
        2 => |s| Box::pin(means_to_an_end::handler(s)),
        _ => {
            tracing::error!("invalid selection");
            panic!("invalid selection");
        }
    }
}
