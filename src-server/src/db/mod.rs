mod schema;
mod operations;

pub use operations::*;
pub use schema::*;

use sqlx::SqlitePool;

pub async fn run_migrations(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(SCHEMA)
        .execute(pool)
        .await?;
    Ok(())
}