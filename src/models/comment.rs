use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

/// Comment status constants (matching Drupal 4.7)
pub const COMMENT_PUBLISHED: i32 = 0;
pub const COMMENT_NOT_PUBLISHED: i32 = 1;

/// Node comment settings
pub const COMMENT_NODE_DISABLED: i32 = 0;
pub const COMMENT_NODE_READ_ONLY: i32 = 1;
pub const COMMENT_NODE_READ_WRITE: i32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Comment {
    pub cid: u32,
    pub pid: u32,
    pub nid: u32,
    pub uid: u32,
    pub subject: String,
    pub comment: String,
    pub hostname: String,
    pub timestamp: i32,
    pub status: i32,
    pub thread: String,
    pub name: Option<String>,
    pub mail: Option<String>,
    pub homepage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CommentWithAuthor {
    pub cid: u32,
    pub pid: u32,
    pub nid: u32,
    pub uid: u32,
    pub subject: String,
    pub comment: String,
    pub hostname: String,
    pub timestamp: i32,
    pub status: i32,
    pub thread: String,
    pub name: Option<String>,
    pub mail: Option<String>,
    pub homepage: Option<String>,
    pub author_name: Option<String>,
    pub depth: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NodeCommentStatistics {
    pub nid: u32,
    pub last_comment_timestamp: i32,
    pub last_comment_name: Option<String>,
    pub last_comment_uid: u32,
    pub comment_count: u32,
}

impl Comment {
    pub async fn find_by_cid(pool: &MySqlPool, cid: u32) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM comments WHERE cid = ?")
            .bind(cid)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_for_node(
        pool: &MySqlPool,
        nid: u32,
        include_unpublished: bool,
    ) -> Result<Vec<CommentWithAuthor>, sqlx::Error> {
        let query = if include_unpublished {
            r#"
            SELECT c.*, u.name as author_name,
                   (LENGTH(c.thread) - LENGTH(REPLACE(c.thread, '.', ''))) as depth
            FROM comments c
            LEFT JOIN users u ON c.uid = u.uid
            WHERE c.nid = ?
            ORDER BY SUBSTRING(c.thread, 1, LENGTH(c.thread) - 1)
            "#
        } else {
            r#"
            SELECT c.*, u.name as author_name,
                   (LENGTH(c.thread) - LENGTH(REPLACE(c.thread, '.', ''))) as depth
            FROM comments c
            LEFT JOIN users u ON c.uid = u.uid
            WHERE c.nid = ? AND c.status = 0
            ORDER BY SUBSTRING(c.thread, 1, LENGTH(c.thread) - 1)
            "#
        };

        sqlx::query_as(query).bind(nid).fetch_all(pool).await
    }

    pub async fn count_for_node(pool: &MySqlPool, nid: u32) -> Result<u32, sqlx::Error> {
        let result: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM comments WHERE nid = ? AND status = 0")
                .bind(nid)
                .fetch_one(pool)
                .await?;
        Ok(result.0 as u32)
    }

    pub async fn create(
        pool: &MySqlPool,
        nid: u32,
        pid: u32,
        uid: u32,
        subject: &str,
        comment: &str,
        hostname: &str,
        name: Option<&str>,
        mail: Option<&str>,
        homepage: Option<&str>,
        status: i32,
    ) -> Result<u32, sqlx::Error> {
        let timestamp = chrono::Utc::now().timestamp() as i32;

        // Calculate thread value
        let thread = Self::calculate_thread(pool, nid, pid).await?;

        let result = sqlx::query(
            r#"
            INSERT INTO comments (nid, pid, uid, subject, comment, hostname, timestamp, status, thread, name, mail, homepage)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(nid)
        .bind(pid)
        .bind(uid)
        .bind(subject)
        .bind(comment)
        .bind(hostname)
        .bind(timestamp)
        .bind(status)
        .bind(&thread)
        .bind(name)
        .bind(mail)
        .bind(homepage)
        .execute(pool)
        .await?;

        let cid = result.last_insert_id() as u32;

        // Update node comment statistics
        Self::update_statistics(pool, nid, uid, name, timestamp).await?;

        Ok(cid)
    }

    pub async fn update(
        pool: &MySqlPool,
        cid: u32,
        subject: &str,
        comment: &str,
        status: i32,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE comments SET subject = ?, comment = ?, status = ? WHERE cid = ?")
            .bind(subject)
            .bind(comment)
            .bind(status)
            .bind(cid)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete(pool: &MySqlPool, cid: u32) -> Result<(), sqlx::Error> {
        // Get comment info for statistics update
        let comment = Self::find_by_cid(pool, cid).await?;

        sqlx::query("DELETE FROM comments WHERE cid = ?")
            .bind(cid)
            .execute(pool)
            .await?;

        // Update statistics if we found the comment
        if let Some(c) = comment {
            Self::recalculate_statistics(pool, c.nid).await?;
        }

        Ok(())
    }

    /// Calculate the thread value for a new comment using vancode encoding
    async fn calculate_thread(pool: &MySqlPool, nid: u32, pid: u32) -> Result<String, sqlx::Error> {
        if pid == 0 {
            // Top-level comment: find max thread at root level
            let result: Option<(String,)> = sqlx::query_as(
                r#"
                SELECT thread FROM comments
                WHERE nid = ? AND pid = 0
                ORDER BY SUBSTRING(thread, 1, LENGTH(thread) - 1) DESC
                LIMIT 1
                "#,
            )
            .bind(nid)
            .fetch_optional(pool)
            .await?;

            let next_num = match result {
                Some((thread,)) => {
                    // Extract the number part (before the /)
                    let num_part = thread.trim_end_matches('/');
                    vancode_to_int(num_part) + 1
                }
                None => 0,
            };

            Ok(format!("{}/", int_to_vancode(next_num)))
        } else {
            // Reply: get parent thread and append new child
            let parent: Option<(String,)> =
                sqlx::query_as("SELECT thread FROM comments WHERE cid = ?")
                    .bind(pid)
                    .fetch_optional(pool)
                    .await?;

            let parent_thread = parent.map(|(t,)| t).unwrap_or_else(|| "00/".to_string());
            let parent_prefix = parent_thread.trim_end_matches('/');

            // Find max child thread under this parent
            let result: Option<(String,)> = sqlx::query_as(
                r#"
                SELECT thread FROM comments
                WHERE nid = ? AND thread LIKE ? AND thread != ?
                ORDER BY thread DESC
                LIMIT 1
                "#,
            )
            .bind(nid)
            .bind(format!("{}.%", parent_prefix))
            .bind(&parent_thread)
            .fetch_optional(pool)
            .await?;

            let next_num = match result {
                Some((thread,)) => {
                    // Find the last segment
                    let child_part = thread.trim_end_matches('/');
                    if let Some(last_dot) = child_part.rfind('.') {
                        let last_segment = &child_part[last_dot + 1..];
                        vancode_to_int(last_segment) + 1
                    } else {
                        0
                    }
                }
                None => 0,
            };

            Ok(format!("{}.{}/", parent_prefix, int_to_vancode(next_num)))
        }
    }

    async fn update_statistics(
        pool: &MySqlPool,
        nid: u32,
        uid: u32,
        name: Option<&str>,
        timestamp: i32,
    ) -> Result<(), sqlx::Error> {
        let count = Self::count_for_node(pool, nid).await?;

        sqlx::query(
            r#"
            INSERT INTO node_comment_statistics (nid, last_comment_timestamp, last_comment_name, last_comment_uid, comment_count)
            VALUES (?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                last_comment_timestamp = ?,
                last_comment_name = ?,
                last_comment_uid = ?,
                comment_count = ?
            "#,
        )
        .bind(nid)
        .bind(timestamp)
        .bind(name)
        .bind(uid)
        .bind(count)
        .bind(timestamp)
        .bind(name)
        .bind(uid)
        .bind(count)
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn recalculate_statistics(pool: &MySqlPool, nid: u32) -> Result<(), sqlx::Error> {
        // Get the latest comment for this node
        let latest: Option<(i32, u32, Option<String>)> = sqlx::query_as(
            r#"
            SELECT timestamp, uid, name FROM comments
            WHERE nid = ? AND status = 0
            ORDER BY timestamp DESC
            LIMIT 1
            "#,
        )
        .bind(nid)
        .fetch_optional(pool)
        .await?;

        let count = Self::count_for_node(pool, nid).await?;

        match latest {
            Some((timestamp, uid, name)) => {
                sqlx::query(
                    r#"
                    INSERT INTO node_comment_statistics (nid, last_comment_timestamp, last_comment_name, last_comment_uid, comment_count)
                    VALUES (?, ?, ?, ?, ?)
                    ON DUPLICATE KEY UPDATE
                        last_comment_timestamp = ?,
                        last_comment_name = ?,
                        last_comment_uid = ?,
                        comment_count = ?
                    "#,
                )
                .bind(nid)
                .bind(timestamp)
                .bind(&name)
                .bind(uid)
                .bind(count)
                .bind(timestamp)
                .bind(&name)
                .bind(uid)
                .bind(count)
                .execute(pool)
                .await?;
            }
            None => {
                // No comments left, reset statistics
                sqlx::query(
                    r#"
                    INSERT INTO node_comment_statistics (nid, last_comment_timestamp, last_comment_name, last_comment_uid, comment_count)
                    VALUES (?, 0, NULL, 0, 0)
                    ON DUPLICATE KEY UPDATE
                        last_comment_timestamp = 0,
                        last_comment_name = NULL,
                        last_comment_uid = 0,
                        comment_count = 0
                    "#,
                )
                .bind(nid)
                .execute(pool)
                .await?;
            }
        }

        Ok(())
    }
}

impl NodeCommentStatistics {
    pub async fn get_for_node(
        pool: &MySqlPool,
        nid: u32,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as("SELECT * FROM node_comment_statistics WHERE nid = ?")
            .bind(nid)
            .fetch_optional(pool)
            .await
    }
}

/// Convert integer to vancode (base-36 with zero-padding)
fn int_to_vancode(i: u32) -> String {
    let mut result = String::new();
    let mut n = i;

    if n == 0 {
        return "00".to_string();
    }

    while n > 0 {
        let digit = (n % 36) as u8;
        let c = if digit < 10 {
            (b'0' + digit) as char
        } else {
            (b'a' + digit - 10) as char
        };
        result.insert(0, c);
        n /= 36;
    }

    // Pad to at least 2 characters
    while result.len() < 2 {
        result.insert(0, '0');
    }

    result
}

/// Convert vancode back to integer
fn vancode_to_int(s: &str) -> u32 {
    let mut result: u32 = 0;
    for c in s.chars() {
        let digit = if c.is_ascii_digit() {
            c as u32 - '0' as u32
        } else if c.is_ascii_lowercase() {
            c as u32 - 'a' as u32 + 10
        } else {
            0
        };
        result = result * 36 + digit;
    }
    result
}
