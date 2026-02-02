use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub uid: u32,
    pub name: String,
    #[serde(skip_serializing)]
    pub pass: String,
    pub mail: Option<String>,
    pub status: i8,
    pub created: i32,
    pub login: i32,
}

impl User {
    pub fn is_anonymous(&self) -> bool {
        self.uid == 0
    }

    pub fn is_authenticated(&self) -> bool {
        self.uid > 0
    }

    pub async fn find_by_uid(pool: &MySqlPool, uid: u32) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE uid = ?")
            .bind(uid)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_name(pool: &MySqlPool, name: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE name = ?")
            .bind(name)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_mail(pool: &MySqlPool, mail: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE mail = ?")
            .bind(mail)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(
        pool: &MySqlPool,
        name: &str,
        pass: &str,
        mail: &str,
    ) -> Result<u32, sqlx::Error> {
        let now = chrono::Utc::now().timestamp() as i32;

        let result = sqlx::query(
            "INSERT INTO users (name, pass, mail, status, created) VALUES (?, ?, ?, 1, ?)",
        )
        .bind(name)
        .bind(pass)
        .bind(mail)
        .bind(now)
        .execute(pool)
        .await?;

        Ok(result.last_insert_id() as u32)
    }

    pub async fn update_login(&self, pool: &MySqlPool) -> Result<(), sqlx::Error> {
        let now = chrono::Utc::now().timestamp() as i32;

        sqlx::query("UPDATE users SET login = ? WHERE uid = ?")
            .bind(now)
            .bind(self.uid)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn update_password(pool: &MySqlPool, uid: u32, pass: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET pass = ? WHERE uid = ?")
            .bind(pass)
            .bind(uid)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn update_mail(pool: &MySqlPool, uid: u32, mail: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE users SET mail = ? WHERE uid = ?")
            .bind(mail)
            .bind(uid)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn get_roles(&self, pool: &MySqlPool) -> Result<Vec<String>, sqlx::Error> {
        let roles: Vec<(String,)> = sqlx::query_as(
            "SELECT r.name FROM role r
             INNER JOIN users_roles ur ON r.rid = ur.rid
             WHERE ur.uid = ?",
        )
        .bind(self.uid)
        .fetch_all(pool)
        .await?;

        Ok(roles.into_iter().map(|(name,)| name).collect())
    }

    pub async fn add_role(pool: &MySqlPool, uid: u32, rid: u32) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT IGNORE INTO users_roles (uid, rid) VALUES (?, ?)")
            .bind(uid)
            .bind(rid)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn has_permission(
        &self,
        pool: &MySqlPool,
        permission: &str,
    ) -> Result<bool, sqlx::Error> {
        if self.uid == 1 {
            return Ok(true);
        }

        let result: Option<(i64,)> = sqlx::query_as(
            "SELECT COUNT(*) FROM permission p
             INNER JOIN users_roles ur ON p.rid = ur.rid
             WHERE ur.uid = ? AND p.perm LIKE ?",
        )
        .bind(self.uid)
        .bind(format!("%{}%", permission))
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|(count,)| count > 0).unwrap_or(false))
    }
}
