use serde::{Deserialize, Serialize};

pub mod alignment;
pub mod gnss;
pub mod magnetic;

pub const BROADCAST_PORT: u16 = 12961;
pub const MAGIC_NUMBER: u32 = 146658626;

#[derive(Serialize, Deserialize)]
pub struct Broadcast {
    pub magic_number: u32,
}
