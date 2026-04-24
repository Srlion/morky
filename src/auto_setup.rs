use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use rand::RngExt;

use crate::{db, models::User};

pub async fn run() {
    let email = std::env::var("MORKY_ADMIN_EMAIL").unwrap_or_default();
    if email.is_empty() {
        return;
    }

    let count: i64 = match db::conn()
        .query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one()
        .await
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("auto_setup: failed to check users: {e}");
            return;
        }
    };

    if count > 0 {
        return;
    }

    let password: String = (0..24)
        .map(|_| {
            const CHARSET: &[u8] =
                b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
            let idx = rand::rng().random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    let salt = SaltString::generate(&mut OsRng);
    let hash = match Argon2::default().hash_password(password.as_bytes(), &salt) {
        Ok(h) => h.to_string(),
        Err(e) => {
            tracing::error!("auto_setup: failed to hash password: {e}");
            return;
        }
    };

    match User::create(&email, "admin", &hash).await {
        Ok(_) => {
            tracing::info!("Admin user created: {email}");
            // Write credentials to a file so the install script can read them
            let creds = format!("{email}\n{password}\n");
            let path = std::path::Path::new(&crate::constants::db_path())
                .parent()
                .unwrap_or(std::path::Path::new("."))
                .join("admin_credentials");
            if let Err(e) = std::fs::write(&path, &creds) {
                tracing::error!("auto_setup: failed to write credentials file: {e}");
            }
        }
        Err(e) => tracing::error!("auto_setup: failed to create user: {e}"),
    }
}
