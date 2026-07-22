use std::sync::LazyLock;

use reqwest::Client;

static CLIENT: LazyLock<Client> = LazyLock::new(|| {
    Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("Failed to create client")
});

pub fn client() -> &'static Client {
    &CLIENT
}
