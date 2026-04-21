use std::{
    collections::HashSet,
    sync::{LazyLock, Mutex},
};

static TOKENS: LazyLock<Mutex<HashSet<[u8; 16]>>> = LazyLock::new(Default::default);

pub fn generate() -> String {
    let bytes: [u8; 16] = rand::random();
    TOKENS.lock().unwrap().insert(bytes);
    crate::hex::encode(&bytes)
}

/// Verifies and consumes the token. Returns `false` if invalid or already used.
pub fn verify(token: &str) -> bool {
    let Some(bytes) = crate::hex::decode(token) else {
        return false;
    };
    let Ok(bytes): Result<[u8; 16], _> = bytes.try_into() else {
        return false;
    };
    TOKENS.lock().unwrap().remove(&bytes)
}
