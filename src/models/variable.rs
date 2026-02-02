use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Variable {
    pub name: String,
    pub value: Option<String>,
}

impl Variable {
    pub async fn get(pool: &MySqlPool, name: &str) -> Result<Option<String>, sqlx::Error> {
        let result: Option<(Option<String>,)> =
            sqlx::query_as("SELECT value FROM variable WHERE name = ?")
                .bind(name)
                .fetch_optional(pool)
                .await?;

        Ok(result.and_then(|(v,)| v))
    }

    pub async fn set(pool: &MySqlPool, name: &str, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO variable (name, value) VALUES (?, ?)
             ON DUPLICATE KEY UPDATE value = VALUES(value)",
        )
        .bind(name)
        .bind(value)
        .execute(pool)
        .await?;
        Ok(())
    }

    pub async fn delete(pool: &MySqlPool, name: &str) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM variable WHERE name = ?")
            .bind(name)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn get_or_default(pool: &MySqlPool, name: &str, default: &str) -> String {
        Self::get(pool, name)
            .await
            .ok()
            .flatten()
            .unwrap_or_else(|| default.to_string())
    }
}
