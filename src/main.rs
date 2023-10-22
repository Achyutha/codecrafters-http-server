use tokio::{net::{TcpListener, TcpStream}, io::{AsyncWriteExt, AsyncReadExt}};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221").await.unwrap();
    loop {
        let (mut socket, _) = listener.accept().await.unwrap();

        process_socket(&mut socket).await;
    }
}


async fn process_socket(socket: &mut TcpStream) {
    socket.read(&mut [0; 128]).await.unwrap();
    socket.write(b"HTTP/1.1 200 OK\r\n\r\n").await.unwrap();
}
