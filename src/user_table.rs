use std::collections::HashMap;
use async_std::sync::{Arc, Mutex};
use async_std::net::TcpStream;

use crate::{send_as_json, ChatResult, FromServer};

type UserTable = Mutex<HashMap<Arc<String>, Arc<TcpStream>>>;
type StreamPtr = Arc<TcpStream>;

pub struct Users(UserTable);

impl Users {
    pub async fn new() -> Users {
        Users(Mutex::new(HashMap::new()))
    }

    pub async fn exists(&self, username: &String) -> bool {
        let table_guard = self.0.lock().await;
        table_guard.contains_key(username)
    }

    pub async fn add_user(&self, username: &String, stream: &TcpStream) -> Option<StreamPtr> {
        let user_ptr = Arc::new(username.clone());
        let stream_ptr = Arc::new(stream.clone());
        self.0.lock().await
            .insert(user_ptr, stream_ptr)
    }

    pub async fn remove_user(&self, username: &String) -> Option<StreamPtr> {
        self.0.lock().await
            .remove(username)
    }

    pub async fn send(&self, username: &String, message: &String) -> ChatResult<()> {
        let message = format!("{} > {}", username, message);
        let bcast_msg = FromServer::Message { message: Arc::new(message.clone()) };
        
        // NOTE: Blocks all streams until done sending
        let table_guard = self.0.lock().await;
        let mut table_iter = table_guard.iter();

        while let Some((uname, stream)) =  table_iter.next() {
            let uname = uname.clone();
            let cur_username = &(*uname); // &Arc -> String -> &String
            if cur_username != username {
                let stream = stream.clone();
                let mut cur_stream = &(*stream); // &Arc -> Stream -> &Stream
                send_as_json(&mut cur_stream, &bcast_msg).await?;
            }
        }
        Ok(())
    }
}

