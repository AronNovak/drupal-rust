use serde::{Deserialize, Serialize};

pub const SESSION_USER_KEY: &str = "user_id";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionData {
    pub uid: u32,
}
