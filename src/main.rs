use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("Logs from your program will appear here!");

    let listener = TcpListener::bind("127.0.0.1:4221").await.unwrap();
    loop {
        match listener.accept().await {
            Ok(_) => {
                println!("acceped new connection");
            },
            Err(e) => {
                println!("error: {}", e);
            }
        };
    }
}


