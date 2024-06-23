use server::*;
mod common;

use common::*;

#[async_std::test]
async fn test_join() -> ChatResult<()> {
    // Launch server and ensure server is ready to recieve clients
    let _server_handle = async_std::task::spawn(launch_server());
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Get socket between client and server
    let client_stream = connect_client_to_server().await?;
    let from_server = send_join(client_stream.clone(), String::from("test-user")).await?;

    assert_eq!(from_server, FromServer::JoinSuccess);

    Ok(())
}

#[async_std::test]
async fn test_reject_same_username() -> ChatResult<()> {
    // Launch server and ensure server is ready to recieve clients
    let _server_handle = async_std::task::spawn(launch_server());
    std::thread::sleep(std::time::Duration::from_secs(1));

    // Get socket between client and server
    let stream1 = connect_client_to_server().await?;
    let from_server = send_join(stream1.clone(), String::from("user1")).await?;

    assert_eq!(from_server, FromServer::JoinSuccess);

    // Try to join using same username
    let stream2 = connect_client_to_server().await?;
    let from_server = send_join(stream2.clone(), String::from("user1")).await?;
    let err_str = format!("'{}' is already taken. Choose another name.", "user1");

    assert_eq!(from_server, FromServer::Err(err_str));
    
    Ok(())
}

