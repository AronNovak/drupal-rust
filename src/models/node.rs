use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Node {
    pub nid: u32,
    pub vid: u32,
    #[sqlx(rename = "type")]
    pub node_type: String,
    pub title: String,
    pub uid: u32,
    pub status: i32,
    pub created: i32,
    pub changed: i32,
    pub promote: i32,
    pub sticky: i32,
    pub comment: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NodeRevision {
    pub vid: u32,
    pub nid: u32,
    pub uid: u32,
    pub title: String,
    pub body: Option<String>,
    pub teaser: Option<String>,
    pub timestamp: i32,
    pub format: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NodeWithBody {
    pub nid: u32,
    pub vid: u32,
    pub node_type: String,
    pub title: String,
    pub uid: u32,
    pub status: i32,
    pub created: i32,
    pub changed: i32,
    pub promote: i32,
    pub sticky: i32,
    pub comment: i32,
    pub body: Option<String>,
    pub teaser: Option<String>,
    pub author_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NodeType {
    #[sqlx(rename = "type")]
    pub type_name: String,
    pub name: String,
    pub description: Option<String>,
    pub help: Option<String>,
}

impl Node {
    pub async fn find_by_nid(pool: &MySqlPool, nid: u32) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Node>("SELECT * FROM node WHERE nid = ?")
            .bind(nid)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_with_body(
        pool: &MySqlPool,
        nid: u32,
    ) -> Result<Option<NodeWithBody>, sqlx::Error> {
        sqlx::query_as::<_, NodeWithBody>(
            "SELECT n.nid, n.vid, n.type as node_type, n.title, n.uid, n.status,
                    n.created, n.changed, n.promote, n.sticky, n.comment,
                    nr.body, nr.teaser, u.name as author_name
             FROM node n
             INNER JOIN node_revisions nr ON n.vid = nr.vid
             LEFT JOIN users u ON n.uid = u.uid
             WHERE n.nid = ?",
        )
        .bind(nid)
        .fetch_optional(pool)
        .await
    }

    pub async fn find_promoted(
        pool: &MySqlPool,
        limit: i32,
    ) -> Result<Vec<NodeWithBody>, sqlx::Error> {
        sqlx::query_as::<_, NodeWithBody>(
            "SELECT n.nid, n.vid, n.type as node_type, n.title, n.uid, n.status,
                    n.created, n.changed, n.promote, n.sticky, n.comment,
                    nr.body, nr.teaser, u.name as author_name
             FROM node n
             INNER JOIN node_revisions nr ON n.vid = nr.vid
             LEFT JOIN users u ON n.uid = u.uid
             WHERE n.status = 1 AND n.promote = 1
             ORDER BY n.sticky DESC, n.created DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn create(
        pool: &MySqlPool,
        node_type: &str,
        title: &str,
        body: &str,
        teaser: &str,
        uid: u32,
        promote: bool,
        sticky: bool,
    ) -> Result<(u32, u32), sqlx::Error> {
        let now = chrono::Utc::now().timestamp() as i32;

        let node_result = sqlx::query(
            "INSERT INTO node (type, title, uid, status, created, changed, promote, sticky)
             VALUES (?, ?, ?, 1, ?, ?, ?, ?)",
        )
        .bind(node_type)
        .bind(title)
        .bind(uid)
        .bind(now)
        .bind(now)
        .bind(if promote { 1 } else { 0 })
        .bind(if sticky { 1 } else { 0 })
        .execute(pool)
        .await?;

        let nid = node_result.last_insert_id() as u32;

        let revision_result = sqlx::query(
            "INSERT INTO node_revisions (nid, uid, title, body, teaser, timestamp)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(nid)
        .bind(uid)
        .bind(title)
        .bind(body)
        .bind(teaser)
        .bind(now)
        .execute(pool)
        .await?;

        let vid = revision_result.last_insert_id() as u32;

        sqlx::query("UPDATE node SET vid = ? WHERE nid = ?")
            .bind(vid)
            .bind(nid)
            .execute(pool)
            .await?;

        Ok((nid, vid))
    }

    pub async fn update(
        pool: &MySqlPool,
        nid: u32,
        title: &str,
        body: &str,
        teaser: &str,
        uid: u32,
        promote: bool,
        sticky: bool,
    ) -> Result<u32, sqlx::Error> {
        let now = chrono::Utc::now().timestamp() as i32;

        sqlx::query("UPDATE node SET title = ?, changed = ?, promote = ?, sticky = ? WHERE nid = ?")
            .bind(title)
            .bind(now)
            .bind(if promote { 1 } else { 0 })
            .bind(if sticky { 1 } else { 0 })
            .bind(nid)
            .execute(pool)
            .await?;

        let revision_result = sqlx::query(
            "INSERT INTO node_revisions (nid, uid, title, body, teaser, timestamp)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(nid)
        .bind(uid)
        .bind(title)
        .bind(body)
        .bind(teaser)
        .bind(now)
        .execute(pool)
        .await?;

        let vid = revision_result.last_insert_id() as u32;

        sqlx::query("UPDATE node SET vid = ? WHERE nid = ?")
            .bind(vid)
            .bind(nid)
            .execute(pool)
            .await?;

        Ok(vid)
    }
}

impl Node {
    pub async fn all_for_admin(pool: &MySqlPool) -> Result<Vec<NodeWithBody>, sqlx::Error> {
        sqlx::query_as::<_, NodeWithBody>(
            "SELECT n.nid, n.vid, n.type as node_type, n.title, n.uid, n.status,
                    n.created, n.changed, n.promote, n.sticky, n.comment,
                    nr.body, nr.teaser, u.name as author_name
             FROM node n
             INNER JOIN node_revisions nr ON n.vid = nr.vid
             LEFT JOIN users u ON n.uid = u.uid
             ORDER BY n.changed DESC",
        )
        .fetch_all(pool)
        .await
    }

    pub async fn delete(pool: &MySqlPool, nid: u32) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM node_field_data WHERE vid IN (SELECT vid FROM node_revisions WHERE nid = ?)")
            .bind(nid)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM node_revisions WHERE nid = ?")
            .bind(nid)
            .execute(pool)
            .await?;
        sqlx::query("DELETE FROM node WHERE nid = ?")
            .bind(nid)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn set_status(pool: &MySqlPool, nid: u32, status: i32) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE node SET status = ? WHERE nid = ?")
            .bind(status)
            .bind(nid)
            .execute(pool)
            .await?;
        Ok(())
    }
}

impl NodeType {
    pub async fn all(pool: &MySqlPool) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, NodeType>("SELECT * FROM node_type ORDER BY name")
            .fetch_all(pool)
            .await
    }

    pub async fn find_by_type(pool: &MySqlPool, type_name: &str) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, NodeType>("SELECT * FROM node_type WHERE type = ?")
            .bind(type_name)
            .fetch_optional(pool)
            .await
    }

    pub async fn update(
        pool: &MySqlPool,
        type_name: &str,
        name: &str,
        description: &str,
        help: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE node_type SET name = ?, description = ?, help = ? WHERE type = ?")
            .bind(name)
            .bind(description)
            .bind(help)
            .bind(type_name)
            .execute(pool)
            .await?;
        Ok(())
    }
}
