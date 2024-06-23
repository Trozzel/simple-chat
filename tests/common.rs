use async_std::io::prelude::BufReadExt;
use async_std::net::TcpStream;
use dotenvy::dotenv;
use server::server_handler::handle_new_clients;
use server::{ChatResult, FromClient, FromServer};
use std::env;

pub async fn launch_server() -> ChatResult<()> {
    dotenv().ok();

    let server_addr = env::var("SERVER_URL")?;
    let server_port = env::var("SERVER_PORT")?;
    let server_url = format!("{}:{}", server_addr, server_port);

    handle_new_clients(server_url).await?;

    Ok(())
}

pub async fn connect_client_to_server() -> ChatResult<TcpStream> {
    let server_addr = env::var("SERVER_URL")?;
    let server_port = env::var("SERVER_PORT")?;
    let server_url = format!("{}:{}", server_addr, server_port);

    let stream = TcpStream::connect(server_url).await?;

    Ok(stream)
}

use async_std::io::BufReader;
use async_std::sync::Arc;
use server::send_as_json;

pub async fn send_join(mut stream: TcpStream, name: String) -> ChatResult<FromServer> {
    // 1. Send JOIN
    let join_server = FromClient::Join {
        username: Arc::new(name),
    };
    send_as_json(&mut stream, &join_server).await?;

    // 2. Receive response
    let mut server_reader = BufReader::new(&stream);
    let mut from_server_str = String::new();
    server_reader.read_line(&mut from_server_str).await?;
    println!("Received {} from server", &from_server_str);

    Ok(serde_json::from_str::<FromServer>(&from_server_str)?)
}
