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

pub fn port() -> u16 {
    std::env::var("PORT")
        .unwrap_or_else(|_| "9764".to_string())
        .parse()
        .unwrap()
}

macro_rules! env_var {
    ($name:ident, $key:literal) => {
        pub fn $name() -> &'static str {
            static V: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
                std::env::var($key).expect(concat!($key, " is required"))
            });
            &V
        }
    };
    ($name:ident, $key:literal, $default:literal) => {
        pub fn $name() -> &'static str {
            static V: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
                std::env::var($key).unwrap_or_else(|_| $default.into())
            });
            &V
        }
    };
}

env_var!(db_path, "DB_PATH");
env_var!(rust_log, "RUST_LOG", "info");
env_var!(host, "APP_HOST", "0.0.0.0");
env_var!(podman_socket, "PODMAN_SOCKET");
env_var!(morky_host_data_dir, "MORKY_HOST_DATA_DIR");
env_var!(morky_data_dir, "MORKY_DATA_DIR");
