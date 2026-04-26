pub mod env_vars;
pub mod hex;
pub mod single_use_token;

mod i_regex;
pub use i_regex::regex;

mod fqdn;
pub use fqdn::is_fqdn;

mod podman;
pub use podman::podman;

mod ttl_map;
pub use ttl_map::TtlMap;

mod defer;
#[allow(unused)]
pub use defer::defer;

mod atomic_write;
pub use atomic_write::atomic_write;

use crate::{constants, http_client};

pub async fn public_base_url() -> Result<String, reqwest::Error> {
    let ip = http_client()
        .get("https://icanhazip.com")
        .send()
        .await?
        .text()
        .await?
        .trim()
        .to_string();
    Ok(format!("http://{ip}:{}", constants::port()))
}

pub trait LogErr<T> {
    #[track_caller]
    fn log_err(self, msg: &str) -> Option<T>;
}

impl<T, E: std::fmt::Display> LogErr<T> for Result<T, E> {
    #[track_caller]
    fn log_err(self, msg: &str) -> Option<T> {
        match self {
            Ok(v) => Some(v),
            Err(e) => {
                let loc = std::panic::Location::caller();
                tracing::error!("{}:{}: {msg}: {e}", loc.file(), loc.line());
                None
            }
        }
    }
}
