pub mod queries;

use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};
use std::str::FromStr;

pub async fn init_db(db_path: &str) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}?mode=rwc", db_path))?
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(options).await?;
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| sqlx::Error::Configuration(e.to_string().into()))?;
    Ok(pool)
}
