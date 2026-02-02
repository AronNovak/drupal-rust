use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProfileField {
    pub fid: u32,
    pub title: Option<String>,
    pub name: String,
    pub explanation: Option<String>,
    pub category: Option<String>,
    pub page: Option<String>,
    #[sqlx(rename = "type")]
    pub field_type: Option<String>,
    pub weight: i8,
    pub required: i8,
    pub register: i8,
    pub visibility: i8,
    pub options: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProfileValue {
    pub fid: u32,
    pub uid: u32,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProfileFieldWithValue {
    pub fid: u32,
    pub title: Option<String>,
    pub name: String,
    pub explanation: Option<String>,
    pub category: Option<String>,
    pub field_type: Option<String>,
    pub weight: i8,
    pub required: i8,
    pub options: Option<String>,
    pub value: Option<String>,
}

impl ProfileField {
    pub async fn all(pool: &MySqlPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, ProfileField>(
            "SELECT * FROM profile_fields ORDER BY category, weight, title",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn for_registration(pool: &MySqlPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, ProfileField>(
            "SELECT * FROM profile_fields WHERE register = 1 ORDER BY category, weight, title",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_fid(pool: &MySqlPool, fid: u32) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, ProfileField>("SELECT * FROM profile_fields WHERE fid = ?")
            .bind(fid)
            .fetch_optional(pool)
            .await
    }

    pub async fn create(
        pool: &MySqlPool,
        title: &str,
        name: &str,
        explanation: Option<&str>,
        category: &str,
        field_type: &str,
        weight: i8,
        required: bool,
        register: bool,
        visibility: i8,
        options: Option<&str>,
    ) -> Result<u32, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO profile_fields (title, name, explanation, category, type, weight, required, register, visibility, options)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(title)
        .bind(name)
        .bind(explanation)
        .bind(category)
        .bind(field_type)
        .bind(weight)
        .bind(if required { 1i8 } else { 0i8 })
        .bind(if register { 1i8 } else { 0i8 })
        .bind(visibility)
        .bind(options)
        .execute(pool)
        .await?;

        Ok(result.last_insert_id() as u32)
    }

    pub fn get_options_list(&self) -> Vec<String> {
        self.options
            .as_ref()
            .map(|o| o.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
            .unwrap_or_default()
    }
}

impl ProfileValue {
    pub async fn get_for_user(
        pool: &MySqlPool,
        uid: u32,
    ) -> Result<Vec<ProfileFieldWithValue>, sqlx::Error> {
        sqlx::query_as::<_, ProfileFieldWithValue>(
            "SELECT pf.fid, pf.title, pf.name, pf.explanation, pf.category,
                    pf.type as field_type, pf.weight, pf.required, pf.options,
                    pv.value
             FROM profile_fields pf
             LEFT JOIN profile_values pv ON pf.fid = pv.fid AND pv.uid = ?
             ORDER BY pf.category, pf.weight, pf.title",
        )
        .bind(uid)
        .fetch_all(pool)
        .await
    }

    pub async fn get_visible_for_user(
        pool: &MySqlPool,
        uid: u32,
        viewer_uid: Option<u32>,
    ) -> Result<Vec<ProfileFieldWithValue>, sqlx::Error> {
        let is_self = viewer_uid.map(|v| v == uid).unwrap_or(false);
        let is_admin = viewer_uid.map(|v| v == 1).unwrap_or(false);

        if is_self || is_admin {
            return Self::get_for_user(pool, uid).await;
        }

        sqlx::query_as::<_, ProfileFieldWithValue>(
            "SELECT pf.fid, pf.title, pf.name, pf.explanation, pf.category,
                    pf.type as field_type, pf.weight, pf.required, pf.options,
                    pv.value
             FROM profile_fields pf
             LEFT JOIN profile_values pv ON pf.fid = pv.fid AND pv.uid = ?
             WHERE pf.visibility > 0
             ORDER BY pf.category, pf.weight, pf.title",
        )
        .bind(uid)
        .fetch_all(pool)
        .await
    }

    pub async fn set(pool: &MySqlPool, fid: u32, uid: u32, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO profile_values (fid, uid, value) VALUES (?, ?, ?)
             ON DUPLICATE KEY UPDATE value = VALUES(value)",
        )
        .bind(fid)
        .bind(uid)
        .bind(value)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete_for_user(pool: &MySqlPool, uid: u32) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM profile_values WHERE uid = ?")
            .bind(uid)
            .execute(pool)
            .await?;

        Ok(())
    }
}
