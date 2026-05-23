//! cargo run -p cms-api --bin seed_admin -- <email> <password>

use argon2::{
    Argon2, PasswordHasher,
    password_hash::{SaltString, rand_core::OsRng},
};
use cms_api::{bootstrap, config, observability, repositories::user_repo};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    observability::init();

    let mut args = std::env::args().skip(1);
    let Some(email) = args.next() else {
        anyhow::bail!("usage: seed_admin <email> <password>");
    };
    let Some(password) = args.next() else {
        anyhow::bail!("usage: seed_admin <email> <password>");
    };

    let cfg = config::load()?;
    let state = bootstrap::AppState::init(cfg).await?;

    if user_repo::find_by_email(&state.db, &email).await?.is_some() {
        println!("user {email} already exists, skipping");
        return Ok(());
    }

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| anyhow::anyhow!("argon2 hash: {error}"))?
        .to_string();
    let user = user_repo::insert(&state.db, email, password_hash, "owner".to_owned()).await?;
    println!("seeded user id={} email={}", user.id, user.email);
    Ok(())
}
