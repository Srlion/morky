use crate::models::User;
use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};

pub async fn setup() -> Result<(), anyhow::Error> {
    crate::db::init().await?;

    let count: i64 = crate::db::conn()
        .query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one()
        .await?;

    if count > 0 {
        println!("already set up, skipping");
        return Ok(());
    }

    let mut args = std::env::args().skip(2);
    let mut email = None;
    let mut password = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--email" => email = args.next(),
            "--password" => password = args.next(),
            _ => {}
        }
    }

    let email = email.ok_or_else(|| anyhow::anyhow!("--email required"))?;
    let password = password.ok_or_else(|| anyhow::anyhow!("--password required"))?;

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("hash: {e}"))?
        .to_string();

    User::create(&email, "admin", &hash).await?;
    Ok(())
}
