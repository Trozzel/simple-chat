use async_std::io::BufReader;
use async_std::net::{TcpListener, TcpStream, ToSocketAddrs};
use async_std::prelude::*;
use async_std::sync::{Arc, Mutex};

use crate::user_table::Users;
use crate::{recv_as_json, send_as_json, ChatResult, ChatState, FromClient, FromServer};

type UserTable = Arc<Mutex<Users>>;

/// Handle an individual client's login attempts
/// ## Return
/// Two-member tuple of:
/// 1. `ChatState`
/// 2. Username string, if successful
async fn handle_waiting_state(
    stream: TcpStream,
    user_table: UserTable,
) -> ChatResult<(ChatState, Option<String>)> {
    // Initialize default return value
    let mut result: (ChatState, Option<String>) = (ChatState::Waiting, None);
    let reader = BufReader::new(&stream);
    let mut from_client_stream = recv_as_json(reader);
    // NOTE: `while` loop iterates only one time and then returns
    while let Some(from_client_result) = from_client_stream.next().await {
        let from_client = from_client_result?;
        match from_client {
            FromClient::Join { username } => {
                // 1. Handle adding to user table
                if user_table.lock().await.exists(&username).await {
                    // No longer need lock on table
                    drop(user_table);

                    let to_client = FromServer::Err(format!(
                        "'{}' is already taken. Choose another name.",
                        &username
                    ));
                    send_as_json(&mut stream.clone(), &to_client).await?;

                // 2. If not exists, add user to table
                } else {
                    // Add user to `Users` table
                    user_table.lock().await.add_user(&username, &stream).await;

                    // Send Success to the client
                    let mut to_client_stream = stream.clone();
                    let to_client = FromServer::JoinSuccess;
                    send_as_json(&mut to_client_stream, &to_client).await?;

                    // Send welcome to the client
                    let to_client = FromServer::Message { message: Arc::new(format!("Welcome {}!", username)) };
                    send_as_json(&mut to_client_stream, &to_client).await?;

                    // Send welcome to other users
                    user_table.lock().await
                        .send(&username, &format!("{} I just entered the chat!", &username)).await?;
                    
                    // Copy username string and add to return value
                    let uname = (*username).clone();
                    result = (ChatState::Joined, Some(uname));
                }
            }
            FromClient::Leave => {
                result = (ChatState::Leaving, None);
            }
            _ => {
                let to_client = FromServer::Err(String::from("Error joining server"));
                send_as_json(&mut stream.clone(), &to_client).await?;
            }
        }
        return Ok(result);
    }
    // If client closes the socket
    println!("Someone took off!");
    Ok((ChatState::Leaving, None))
}

/// Handle an individual clients interaction with server after joined
async fn handle_joined_state(
    stream: TcpStream,
    username: &String,
    user_table: UserTable,
) -> ChatResult<ChatState> {
    // Initialize `ChatState` to minimize return points
    let mut chat_state = ChatState::Joined;

    let reader = BufReader::new(&stream);
    let mut json_stream = recv_as_json(reader);
    // NOTE: `while` loop iterates only one time and then returns
    while let Some(from_client_result) = json_stream.next().await {
        let from_client = from_client_result?;
        match from_client {
            // `FromClient::Join` should be impossible from the client side
            FromClient::Join { .. } => {
                let to_client = FromServer::Err(String::from("You're already joined."));
                send_as_json(&mut stream.clone(), &to_client).await?;
            }
            // Send message to all other users
            FromClient::Send { message } => {
                user_table
                    .lock()
                    .await
                    .send(&username, &(*&message))
                    .await?;
            }
            // Remove user from table
            FromClient::Leave => {
                // 1. Let users know
                let users_guard = user_table.lock().await;
                users_guard
                    .send(&username, &String::from("Outa-here like Vladamir!"))
                    .await?;

                // 2. Remove user from user table
                let _ = users_guard.remove_user(username).await;

                println!("User is leaving the chat");
                chat_state = ChatState::Leaving;
            }
        }
        return Ok(chat_state);
    }
    // Someone rudely closed the stream
    let table_guard = user_table.lock().await;
    table_guard
        .send(username, &String::from("Later guys!"))
        .await?;
    table_guard.remove_user(username).await;
    Ok(ChatState::Leaving)
}

/// Represents an individual client loop
/// NOTE: copying streams is equivilant of cloning pointers (i.e. 64-bytes),
/// therefore a huge issue. However, consider maintaining a primary stream
/// for each state.
async fn client_state_machine(stream: TcpStream, user_table: UserTable) -> ChatResult<()> {
    let mut username = String::new();
    let mut chat_state = ChatState::Waiting;
    loop {
        match chat_state {
            ChatState::Waiting => {
                let (new_state, uname_op) =
                    handle_waiting_state(stream.clone(), user_table.clone()).await?;
                if uname_op.is_some() {
                    username = uname_op.unwrap();
                }
                chat_state = new_state;
            }
            ChatState::Joined => {
                chat_state =
                    handle_joined_state(stream.clone(), &username, user_table.clone()).await?;
            }
            ChatState::Leaving => {
                println!("Client is leaving the chat room.");
                break;
            }
        }
    }
    Ok(())
}

/// Receive client socket and sends to `client state machine`
/// This function is called in a `async_std::task::block_on` to initiate the
/// client
pub async fn handle_new_clients(addr: impl ToSocketAddrs) -> ChatResult<()> {
    // Initiate client user table
    let user_table = Arc::new(Mutex::new(Users::new().await));

    let listener = TcpListener::bind(addr).await?;
    let mut incoming = listener.incoming();
    while let Some(stream_result) = incoming.next().await {
        let stream = stream_result?;
        println!("Accepting from {}", stream.peer_addr()?);

        // Handle new client
        let _handle = async_std::task::spawn(client_state_machine(stream, user_table.clone()));
    }
    Ok(())
}


