pub fn db_path() -> String {
    std::env::var("DB_PATH").expect("DB_PATH is required")
}

pub fn cookie_secret_key() -> String {
    static KEY: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
        let path = std::path::Path::new(&db_path())
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("secret_key");
        if let Ok(k) = std::fs::read_to_string(&path) {
            return k.trim().to_string();
        }
        let key = crate::common::hex::encode(&rand::random::<[u8; 64]>());
        let _ = std::fs::write(&path, &key);
        key
    });
    KEY.clone()
}

pub fn rust_log() -> String {
    std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string())
}

pub fn host() -> String {
    std::env::var("APP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string())
}

pub fn port() -> u16 {
    std::env::var("PORT")
        .unwrap_or_else(|_| "9764".to_string())
        .parse()
        .unwrap()
}
