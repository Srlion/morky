pub fn is_prod() -> bool {
    envd::dyn_var!("RUST_ENV").starts_with("prod")
}

pub fn db_path() -> String {
    envd::dyn_var!("DB_PATH")
}

pub fn cookie_secret_key() -> String {
    envd::dyn_var!("COOKIE_SECRET_KEY")
}

pub fn rust_log() -> String {
    envd::dyn_var!("RUST_LOG")
}

pub fn port() -> u16 {
    envd::var!("PORT": u16)
}
