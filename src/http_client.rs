use std::sync::LazyLock;

use reqwest::Client;

static CLIENT: LazyLock<Client> =
    LazyLock::new(|| Client::builder().build().expect("Failed to create client"));

pub fn client() -> &'static Client {
    &CLIENT
}
