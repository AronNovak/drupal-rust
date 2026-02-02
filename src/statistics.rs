use axum::{
    body::Body,
    extract::State,
    http::Request,
    middleware::Next,
    response::Response,
};
use sqlx::MySqlPool;
use std::time::Instant;

use crate::models::{AccessLog, NodeCounter, SystemItem, Variable};

pub async fn statistics_middleware(
    State(pool): State<MySqlPool>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let start = Instant::now();
    let path = request.uri().path().to_string();
    let method = request.method().clone();

    // Get request info before consuming it
    let headers = request.headers().clone();
    let referer = headers
        .get("referer")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();
    let host = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|h| h.to_str().ok())
        .unwrap_or("127.0.0.1")
        .to_string();

    // Get session ID if present
    let session_id = headers
        .get("cookie")
        .and_then(|h| h.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let c = c.trim();
                if c.starts_with("id=") {
                    Some(c[3..].to_string())
                } else {
                    None
                }
            })
        })
        .unwrap_or_default();

    // Call the next handler
    let response = next.run(request).await;

    // Only log GET requests for non-static paths
    if method == "GET" && !path.starts_with("/static") {
        let timer = start.elapsed().as_millis() as u32;

        // Spawn a task to log the access (don't block the response)
        let pool_clone = pool.clone();
        let path_clone = path.clone();
        tokio::spawn(async move {
            // Check if statistics module is enabled
            if !SystemItem::is_module_enabled(&pool_clone, "statistics")
                .await
                .unwrap_or(false)
            {
                return;
            }

            // Check if access logging is enabled
            let log_enabled = Variable::get(&pool_clone, "statistics_enable_access_log")
                .await
                .ok()
                .flatten()
                .map(|v| v == "1")
                .unwrap_or(false);

            if log_enabled {
                // Get title from path (simplified - just use path for now)
                let title = path_clone.clone();

                let _ = AccessLog::log_access(
                    &pool_clone,
                    &session_id,
                    &title,
                    &path_clone,
                    &referer,
                    &host,
                    0, // uid - would need to extract from session
                    timer,
                )
                .await;
            }

            // Check if node counter is enabled and path is a node view
            let count_enabled = Variable::get(&pool_clone, "statistics_count_content_views")
                .await
                .ok()
                .flatten()
                .map(|v| v == "1")
                .unwrap_or(false);

            if count_enabled && path_clone.starts_with("/node/") {
                // Extract node ID from path like /node/123
                if let Some(nid_str) = path_clone.strip_prefix("/node/") {
                    if let Ok(nid) = nid_str.parse::<u32>() {
                        let _ = NodeCounter::increment(&pool_clone, nid).await;
                    }
                }
            }
        });
    }

    response
}
