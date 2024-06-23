use serde::Serialize;

use async_std::net::TcpStream;
use async_std::prelude::*;
use async_std::io::BufReader;
use async_std::sync::Arc;

use crate::{recv_as_json, send_as_json, ChatResult, ChatState, FromClient, FromServer};

/// Handles join attempts to the server
/// Only called from within `handle_waiting_state`
async fn handle_join_with_server(
    stream: &TcpStream,
    data: &impl Serialize,
) -> ChatResult<ChatState> {
    // Initialize return value
    let mut state = ChatState::Waiting;
    // 1. Send the data
    send_as_json(&mut stream.clone(), data).await?;

    // 2. Receive status from the server.
    let reader = BufReader::new(stream.clone());
    let mut json_stream = recv_as_json(reader);
    while let Some(from_server_result) = json_stream.next().await {
        let from_server = from_server_result?;
        match from_server {
            FromServer::JoinSuccess => {
                state = ChatState::Joined;
            }
            FromServer::Err(err) => {
                eprintln!("Error from server: {}", err);
            }
            _ => {
                eprintln!("Should be impossible to get here");
            }
        }
        return Ok(state);
    }

    Ok(ChatState::Waiting)
}

/// The WAITING state
/// Manages client's attempt to join to the server
async fn handle_waiting_state(stream: &TcpStream) -> ChatResult<(ChatState, Option<String>)> {
    // Initialize return value
    let mut result: (ChatState, Option<String>) = (ChatState::Waiting, None);

    let mut cmd_line_reader = BufReader::new(async_std::io::stdin()).lines();
    while let Some(line_result) = cmd_line_reader.next().await {
        let line = line_result?;
        match parse_cmd(&line) {
            Some(FromClient::Join { username }) => {
                let join_chat = FromClient::Join {
                    username: username.clone(),
                };
                let join_result = handle_join_with_server(stream, &join_chat).await?;
                if let ChatState::Joined = join_result {
                    result = (ChatState::Joined, Some((*username).clone()));
                }
            }
            Some(FromClient::Leave) => {
                println!("Bye-bye...");
                result = (ChatState::Leaving, None);
            }
            _ => (),
        }
        return Ok(result);
    }
    Ok((ChatState::Waiting, None))
}

/// The JOINED state
/// Manages client's message sending and leaving
async fn handle_joined_state(stream: &TcpStream) -> ChatResult<ChatState> {
    let mut state = ChatState::Joined;
    let mut cmd_line_reader = BufReader::new(async_std::io::stdin()).lines();

    // Read line from stdin
    while let Some(line_result) = cmd_line_reader.next().await {
        let line = line_result?;
        match parse_cmd(&line) {
            Some(FromClient::Send { message }) => {
                let to_server = FromClient::Send {
                    message: message.clone(),
                };
                send_as_json(&mut stream.clone(), &to_server).await?;
            }
            Some(FromClient::Join { username: _ }) => {
                eprintln!("You are already joined.");
            }
            Some(FromClient::Leave) => {
                send_as_json(&mut stream.clone(), &(FromClient::Leave)).await?;
                // No need to await response
                state = ChatState::Leaving;
            }
            _ => ()
        }
        return Ok(state);
    }
    Ok(state)
}

/// Server STATE MACHINE
/// Serves three (3) states:
/// 1. `ChatState::Waiting`
/// 2. `ChatState::Joined`
/// 3. `ChatState::Leaving`
pub async fn client_state_machine(stream: TcpStream) -> ChatResult<()> {
    let mut chat_state = ChatState::Waiting;
    //let mut username = String::new();
    loop {
        match chat_state {
            ChatState::Waiting => {
                let (new_chat_state, _uname_op) = handle_waiting_state(&stream).await?;
                chat_state = new_chat_state;
            }
            ChatState::Joined => {
                chat_state = handle_joined_state(&stream).await?;
            }
            ChatState::Leaving => {
                println!("You are in Leaving state");
                break;
            }
        }
    }
    Ok(())
}

/// Receives messages from server and prints to stdout
pub async fn handle_incoming(stream: TcpStream) -> ChatResult<()> {
    let reader = BufReader::new(stream);
    let mut json_stream = recv_as_json(reader);
    while let Some(from_server_result) = json_stream.next().await {
        let from_server = from_server_result?;
        match from_server {
            FromServer::Message { message } => println!("{}", message),
            FromServer::Err(err) => eprintln!("From server: {}", err),
            _ => (),
        }
    }
    Ok(())
}

/// Parse inputs from the command line to return `FromClient` objects
/// TODO: This isn't the best parser ever
pub fn parse_cmd(line: &str) -> Option<FromClient> {
    // Handle input from the command line
    let no_whitespace = line.trim();
    let mut cmd_iter = no_whitespace.split(' ');
    match cmd_iter.next()?.trim() {
        "join" => {
            // If the user has yet to join, send `FromClient::Join` while
            // handling an input of `$ join` with no username
            if let Some(username) = Some(cmd_iter.next()?.to_string()) {
                Some(FromClient::Join {
                    username: Arc::new(username),
                })
            }
            // If no username was provided
            else {
                eprintln!("To join, : 'join <username>'");
                None
            }
        }
        "send" => {
            // Case in which user tries to send a message but hasn't yet
            // logged in
            let message: String = cmd_iter.map(|token| format!("{} ", token))
                .collect();
            let message = message.trim().to_string();
            Some(FromClient::Send {
                message: Arc::new(message),
            })
        }
        "leave" => Some(FromClient::Leave),
        _ => {
            eprintln!("Invalid command");
            return None;
        }
    }
}

// Unit testing
/******************************************************************************/
#[cfg(test)]
mod tests {
    use crate::FromClient;
    use async_std::sync::Arc;

    use super::*;

    #[test]
    fn test_join_cmd() {
        // Join
        let line1 = String::from("join frank");
        let from_client1 = FromClient::Join { username: Arc::new("frank".to_string()) };
        let parsed1 = parse_cmd(&line1).unwrap();
        assert_eq!(from_client1, parsed1);
    }

    #[test]
    fn test_send_cmd() {
        // Send
        let line2 = String::from("send my message");
        let from_client2 = FromClient::Send { message: Arc::new("my message".to_string()) };
        let parsed2 = parse_cmd(&line2).unwrap();
        assert_eq!(from_client2, parsed2);
    }

    #[test]
    fn test_leave_cmd() {
        // Leave
        let line3 = String::from("leave");
        let from_client3 = FromClient::Leave;
        let parsed3 = parse_cmd(&line3).unwrap();
        assert_eq!(from_client3, parsed3);
    }
}
