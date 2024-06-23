use dotenvy::dotenv;

use server::server_handler::handle_new_clients;
use server::{get_server_url, ChatResult};

/// Lanuch server
fn main() -> ChatResult<()> {
    dotenv().ok();

    let server_url = get_server_url()?;
    let _ = async_std::task::block_on(handle_new_clients(server_url));

    println!("Server is shutting down...");
    Ok(())
}

// Unit testing
/******************************************************************************/
#[cfg(test)]
mod tests {
    use dotenvy::dotenv;
    use regex::Regex;
    use server::ChatResult;
    use std::env;

    #[test]
    fn test_ip_from_env() -> ChatResult<()> {
        dotenv().ok();

        let server_addr = env::var("SERVER_URL")?;
        // NOTE: Provided by Brave Browser AI
        let re = Regex::new(r"^((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$").unwrap();
        assert!(re.is_match(&server_addr));

        Ok(())
    }

    #[test]
    fn test_port_from_env() -> ChatResult<()> {
        dotenv().ok();

        let port_str = env::var("SERVER_PORT")?;
        let port_num = u32::from_str_radix(&port_str, 10)?;
        // NOTE: Could choose another min value for port number
        assert!((8000 <= port_num) && (port_num <= 65535));

        Ok(())
    }
}
