use async_std::prelude::*;
use async_std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// `ChatResult` for handling generic `Result` types
pub type ChatError = Box<dyn Error + Sync + Send + 'static>;
pub type ChatResult<T> = Result<T, ChatError>;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum FromClient {
    Join { username: Arc<String> },
    Send { message: Arc<String> },
    Leave,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum FromServer {
    JoinSuccess,
    Message { message: Arc<String> },
    Err(String),
}

#[derive(Debug)]
pub enum ChatState {
    Waiting,
    Joined,
    Leaving,
}


/// Acquire the server URL (<address>:<port>) either  through the command line
/// or fallback to the environment variables in the `.env` file
pub fn get_server_url() -> ChatResult<String> {
    let server_addr: String;
    let server_port: String;
    let args: Vec<String> = std::env::args().skip(1).collect();

    if args.is_empty() {
        server_addr = std::env::var("SERVER_URL")?;
        server_port = std::env::var("SERVER_PORT")?;
    }
    else if args.len() == 2 {
        server_addr = args[0].clone();
        server_port = args[1].clone();
    }
    else{
        return Err(ChatError::from(String::from("Formatting error at cmd line")));
    }
    Ok(format!("{}:{}", server_addr, server_port))
}


/// Send `serde_json` serializable objects to a `Writer`
/// NOTE: all JSON objects have a newline appended since the stream reader 
/// is triggered to read by newlines
pub async fn send_as_json<W, T>(writer: &mut W, data: &T) -> ChatResult<()>
where
    W: async_std::io::Write + Unpin,
    T: Serialize,
{
    let mut json = serde_json::to_string(data)?;
    json.push('\n');
    writer.write_all(&json.as_bytes()).await?;
    Ok(())
}

use serde::de::DeserializeOwned;

/// Receives data from an async Reader and returns an iterable `Stream`.
/// ## Parameters:
/// - `reader`: Any async BufReader (specifically a `TcpStream`)
/// ## Return:
/// A `Stream` *iterator* through which `ChatResult<FromServer | FromClient>`
/// objects are received
/// Notes:
/// Not async
pub fn recv_as_json<S, P>(reader: S) -> impl Stream<Item = ChatResult<P>>
where
    S: async_std::io::BufRead + Unpin,
    P: DeserializeOwned,
{
    reader.lines().map(|line_res| -> ChatResult<P> {
        let line = line_res?;
        match serde_json::from_str::<P>(&line) {
            Ok(parsed) => Ok(parsed),
            Err(parse_err) => {
                eprintln!("Error parsing data: {}", parse_err);
                Err(Box::new(parse_err))
            }
        }
    })
}

pub mod user_table;
pub mod client_handler;
pub mod server_handler;

// Unit testing
/******************************************************************************/
#[cfg(test)]
pub mod tests {
    use std::sync::Arc;
    use super::*;

    #[test]
    fn test_send_from_client() -> ChatResult<()> {
        let from_client1 = FromClient::Send { message: Arc::new(String::from("message1")) };
        let json1 = r#"{"Send":{"message":"message1"}}"#.to_string();
        assert_eq!(serde_json::to_string(&from_client1)?, json1);

        Ok(())
    }

    #[test]
    fn test_join_from_client() -> ChatResult<()> {
        let from_client2 = FromClient::Join { username: Arc::new(String::from("buddy")) };
        let json2 = r#"{"Join":{"username":"buddy"}}"#.to_string();
        assert_eq!(serde_json::to_string(&from_client2)?, json2);
        Ok(())
    }

    #[test]
    fn test_leave_from_client() -> ChatResult<()> {
        let from_client = FromClient::Leave;
        let json = r#""Leave""#.to_string();
        assert_eq!(serde_json::to_string(&from_client)?, json);
        Ok(())
    }
}
