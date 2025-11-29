use tokio::io::stdin;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server_addr = "127.0.0.1:8080";

    println!("Connecting to server {}...", server_addr);

    let stream = TcpStream::connect(server_addr).await?;
    println!("Connected! Type messages and press Enter.");

    // Create a channel for forwarding input from stdin → network
    let (tx, mut rx) = mpsc::channel::<String>(100);

    // TASK 1: Read user input and send messages through channel
    task::spawn(async move {
        let mut reader = BufReader::new(stdin());
        let mut input = String::new();
        loop {
            input.clear();
            let bytes = reader.read_line(&mut input).await.unwrap();
            if bytes == 0 {
                break;
            }

            if tx.send(input.clone()).await.is_err() {
                break;
            }
        }
    });

    // TASK 2: Receive messages from server
    let (mut read_half, mut write_half) = stream.into_split();
    task::spawn(async move {
        let mut buffer = vec![0u8; 1024];
        loop {
            match read_half.read(&mut buffer).await {
                Ok(0) => {
                    println!("Server closed connection.");
                    std::process::exit(0);
                }
                Ok(n) => {
                    print!("> {}", String::from_utf8_lossy(&buffer[..n]));
                }
                Err(_) => {
                    println!("Error reading from server.");
                    std::process::exit(0);
                }
            }
        }
    });

    // MAIN LOOP: Take input from channel and write to server
    while let Some(msg) = rx.recv().await {
        // Write the broadcasted message to the client
        if write_half.write_all(msg.as_bytes()).await.is_err() {
            // Client socket write failed (client disconnected forcefully)
            break;
        }

        write_half.flush().await?;
    }

    Ok(())
}
