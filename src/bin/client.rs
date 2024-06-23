use async_std::net::TcpStream;
use async_std::prelude::*;
use dotenvy::dotenv;
use server::{ChatResult, get_server_url};
use server::client_handler::{client_state_machine, handle_incoming};

/// Lanches two async tasks:
/// 1. Handle outgoing messages to the server
/// 2. Handle incoming messages from the server
fn main() -> ChatResult<()> {
    dotenv().ok();

    let server_url = get_server_url()?;

    async_std::task::block_on(async {
        let stream = TcpStream::connect(&server_url).await?;
        let outgoing = client_state_machine(stream.clone());
        let incoming = handle_incoming(stream.clone());

        // If any task ends, the process is terminated
        outgoing.race(incoming).await?;
        Ok(())
    })
}

// Unit testing
/******************************************************************************/
#[cfg(test)]
mod tests {
    use server::FromClient;
    use server::client_handler::parse_cmd;
    use async_std::sync::Arc;
    

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
