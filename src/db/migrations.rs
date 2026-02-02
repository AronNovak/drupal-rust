use sqlx::MySqlPool;

const SCHEMA: &str = include_str!("../../sql/schema.sql");

pub async fn run_migrations(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    tracing::info!("Running database migrations...");

    for statement in SCHEMA.split(';') {
        let statement = statement.trim();
        if statement.is_empty() || statement.starts_with("--") {
            continue;
        }

        sqlx::query(statement).execute(pool).await?;
    }

    tracing::info!("Migrations completed successfully");
    Ok(())
}

pub async fn is_installed(pool: &MySqlPool) -> Result<bool, sqlx::Error> {
    let result: Option<(i64,)> = sqlx::query_as(
        "SELECT COUNT(*) FROM users WHERE uid > 0 AND status = 1",
    )
    .fetch_optional(pool)
    .await?;

    Ok(result.map(|(count,)| count > 0).unwrap_or(false))
}
