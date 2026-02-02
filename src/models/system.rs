use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SystemItem {
    pub filename: String,
    pub name: String,
    #[sqlx(rename = "type")]
    pub item_type: String,
    pub description: Option<String>,
    pub status: i32,
    pub throttle: i8,
    pub bootstrap: i32,
    pub schema_version: i16,
    pub weight: i32,
}

impl SystemItem {
    pub async fn all_modules(pool: &MySqlPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, SystemItem>(
            "SELECT * FROM system WHERE type = 'module' ORDER BY weight, name",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn enabled_modules(pool: &MySqlPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, SystemItem>(
            "SELECT * FROM system WHERE type = 'module' AND status = 1 ORDER BY weight, name",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn all_themes(pool: &MySqlPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, SystemItem>(
            "SELECT * FROM system WHERE type = 'theme' ORDER BY name",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn enabled_themes(pool: &MySqlPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, SystemItem>(
            "SELECT * FROM system WHERE type = 'theme' AND status = 1 ORDER BY name",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_name(
        pool: &MySqlPool,
        name: &str,
        item_type: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, SystemItem>(
            "SELECT * FROM system WHERE name = ? AND type = ?",
        )
        .bind(name)
        .bind(item_type)
        .fetch_optional(pool)
        .await
    }

    pub async fn set_status(
        pool: &MySqlPool,
        name: &str,
        item_type: &str,
        status: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE system SET status = ? WHERE name = ? AND type = ?")
            .bind(status)
            .bind(name)
            .bind(item_type)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn is_module_enabled(pool: &MySqlPool, name: &str) -> Result<bool, sqlx::Error> {
        let result: Option<(i32,)> = sqlx::query_as(
            "SELECT status FROM system WHERE name = ? AND type = 'module'",
        )
        .bind(name)
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|(status,)| status == 1).unwrap_or(false))
    }

    pub async fn enable_module(pool: &MySqlPool, name: &str) -> Result<(), sqlx::Error> {
        Self::set_status(pool, name, "module", 1).await
    }

    pub async fn disable_module(pool: &MySqlPool, name: &str) -> Result<(), sqlx::Error> {
        Self::set_status(pool, name, "module", 0).await
    }

    pub async fn enable_theme(pool: &MySqlPool, name: &str) -> Result<(), sqlx::Error> {
        Self::set_status(pool, name, "theme", 1).await
    }

    pub async fn disable_theme(pool: &MySqlPool, name: &str) -> Result<(), sqlx::Error> {
        Self::set_status(pool, name, "theme", 0).await
    }

    pub fn is_required_module(&self) -> bool {
        matches!(
            self.name.as_str(),
            "system" | "node" | "user" | "filter" | "block"
        )
    }
}

pub async fn get_default_theme(pool: &MySqlPool) -> String {
    crate::models::Variable::get_or_default(pool, "theme_default", "bluemarine").await
}

pub async fn set_default_theme(pool: &MySqlPool, theme: &str) -> Result<(), sqlx::Error> {
    crate::models::Variable::set(pool, "theme_default", theme).await
}
