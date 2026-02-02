use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AccessLog {
    pub aid: u32,
    pub sid: String,
    pub title: Option<String>,
    pub path: Option<String>,
    pub url: Option<String>,
    pub hostname: Option<String>,
    pub uid: u32,
    pub timer: u32,
    pub timestamp: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AccessLogWithUser {
    pub aid: u32,
    pub sid: String,
    pub title: Option<String>,
    pub path: Option<String>,
    pub url: Option<String>,
    pub hostname: Option<String>,
    pub uid: u32,
    pub timer: u32,
    pub timestamp: u32,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct NodeCounter {
    pub nid: u32,
    pub totalcount: u64,
    pub daycount: u32,
    pub timestamp: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TopPage {
    pub path: Option<String>,
    pub title: Option<String>,
    pub hits: i64,
    pub total_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TopVisitor {
    pub hostname: Option<String>,
    pub uid: u32,
    pub username: Option<String>,
    pub hits: i64,
    pub total_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TopReferrer {
    pub url: Option<String>,
    pub hits: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PopularNode {
    pub nid: u32,
    pub title: String,
    pub totalcount: u64,
    pub daycount: u32,
    pub timestamp: u32,
}

impl AccessLog {
    pub async fn log_access(
        pool: &MySqlPool,
        sid: &str,
        title: &str,
        path: &str,
        url: &str,
        hostname: &str,
        uid: u32,
        timer: u32,
    ) -> Result<(), sqlx::Error> {
        let timestamp = chrono::Utc::now().timestamp() as u32;

        sqlx::query(
            "INSERT INTO accesslog (sid, title, path, url, hostname, uid, timer, timestamp)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(sid)
        .bind(title)
        .bind(path)
        .bind(url)
        .bind(hostname)
        .bind(uid)
        .bind(timer)
        .bind(timestamp)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn recent_hits(pool: &MySqlPool, limit: i32) -> Result<Vec<AccessLogWithUser>, sqlx::Error> {
        sqlx::query_as::<_, AccessLogWithUser>(
            "SELECT a.*, u.name as username
             FROM accesslog a
             LEFT JOIN users u ON a.uid = u.uid
             ORDER BY a.timestamp DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn find_by_aid(pool: &MySqlPool, aid: u32) -> Result<Option<AccessLogWithUser>, sqlx::Error> {
        sqlx::query_as::<_, AccessLogWithUser>(
            "SELECT a.*, u.name as username
             FROM accesslog a
             LEFT JOIN users u ON a.uid = u.uid
             WHERE a.aid = ?",
        )
        .bind(aid)
        .fetch_optional(pool)
        .await
    }

    pub async fn top_pages(pool: &MySqlPool, limit: i32) -> Result<Vec<TopPage>, sqlx::Error> {
        sqlx::query_as::<_, TopPage>(
            "SELECT path, title, COUNT(*) as hits, COALESCE(SUM(timer), 0) as total_time
             FROM accesslog
             GROUP BY path, title
             ORDER BY hits DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn top_visitors(pool: &MySqlPool, limit: i32) -> Result<Vec<TopVisitor>, sqlx::Error> {
        sqlx::query_as::<_, TopVisitor>(
            "SELECT a.hostname, a.uid, u.name as username, COUNT(*) as hits, COALESCE(SUM(a.timer), 0) as total_time
             FROM accesslog a
             LEFT JOIN users u ON a.uid = u.uid
             GROUP BY a.hostname, a.uid, u.name
             ORDER BY hits DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn top_referrers(pool: &MySqlPool, limit: i32) -> Result<Vec<TopReferrer>, sqlx::Error> {
        sqlx::query_as::<_, TopReferrer>(
            "SELECT url, COUNT(*) as hits
             FROM accesslog
             WHERE url IS NOT NULL AND url != '' AND url NOT LIKE '%://localhost%' AND url NOT LIKE '%://127.0.0.1%'
             GROUP BY url
             ORDER BY hits DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn user_history(pool: &MySqlPool, uid: u32, limit: i32) -> Result<Vec<AccessLogWithUser>, sqlx::Error> {
        sqlx::query_as::<_, AccessLogWithUser>(
            "SELECT a.*, u.name as username
             FROM accesslog a
             LEFT JOIN users u ON a.uid = u.uid
             WHERE a.uid = ?
             ORDER BY a.timestamp DESC
             LIMIT ?",
        )
        .bind(uid)
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn flush_old_entries(pool: &MySqlPool, max_age: u32) -> Result<u64, sqlx::Error> {
        let cutoff = chrono::Utc::now().timestamp() as u32 - max_age;

        let result = sqlx::query("DELETE FROM accesslog WHERE timestamp < ?")
            .bind(cutoff)
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
    }
}

impl NodeCounter {
    pub async fn increment(pool: &MySqlPool, nid: u32) -> Result<(), sqlx::Error> {
        let timestamp = chrono::Utc::now().timestamp() as u32;

        sqlx::query(
            "INSERT INTO node_counter (nid, totalcount, daycount, timestamp)
             VALUES (?, 1, 1, ?)
             ON DUPLICATE KEY UPDATE
                totalcount = totalcount + 1,
                daycount = daycount + 1,
                timestamp = VALUES(timestamp)",
        )
        .bind(nid)
        .bind(timestamp)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get(pool: &MySqlPool, nid: u32) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, NodeCounter>("SELECT * FROM node_counter WHERE nid = ?")
            .bind(nid)
            .fetch_optional(pool)
            .await
    }

    pub async fn reset_day_counts(pool: &MySqlPool) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE node_counter SET daycount = 0")
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn delete_for_node(pool: &MySqlPool, nid: u32) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM node_counter WHERE nid = ?")
            .bind(nid)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn popular_today(pool: &MySqlPool, limit: i32) -> Result<Vec<PopularNode>, sqlx::Error> {
        sqlx::query_as::<_, PopularNode>(
            "SELECT nc.nid, n.title, nc.totalcount, nc.daycount, nc.timestamp
             FROM node_counter nc
             INNER JOIN node n ON nc.nid = n.nid
             WHERE n.status = 1 AND nc.daycount > 0
             ORDER BY nc.daycount DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn popular_all_time(pool: &MySqlPool, limit: i32) -> Result<Vec<PopularNode>, sqlx::Error> {
        sqlx::query_as::<_, PopularNode>(
            "SELECT nc.nid, n.title, nc.totalcount, nc.daycount, nc.timestamp
             FROM node_counter nc
             INNER JOIN node n ON nc.nid = n.nid
             WHERE n.status = 1
             ORDER BY nc.totalcount DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }

    pub async fn recently_viewed(pool: &MySqlPool, limit: i32) -> Result<Vec<PopularNode>, sqlx::Error> {
        sqlx::query_as::<_, PopularNode>(
            "SELECT nc.nid, n.title, nc.totalcount, nc.daycount, nc.timestamp
             FROM node_counter nc
             INNER JOIN node n ON nc.nid = n.nid
             WHERE n.status = 1
             ORDER BY nc.timestamp DESC
             LIMIT ?",
        )
        .bind(limit)
        .fetch_all(pool)
        .await
    }
}
