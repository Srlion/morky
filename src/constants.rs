pub fn is_prod() -> bool {
    envd::dyn_var!("RUST_ENV").starts_with("prod")
}

pub fn db_path() -> String {
    envd::dyn_var!("DB_PATH")
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
    envd::dyn_var!("RUST_LOG")
}

pub fn port() -> u16 {
    envd::var!("PORT": u16)
}
