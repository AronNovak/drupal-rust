mod auth;
mod config;
mod db;
mod error;
mod extractors;
mod handlers;
mod models;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use sqlx::MySqlPool;
use std::sync::Arc;
use tera::Tera;
use tower_http::services::ServeDir;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::MySqlStore;

use crate::auth::auth_middleware;
use crate::config::Config;

fn format_date_filter(
    value: &tera::Value,
    _args: &std::collections::HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let timestamp = match value {
        tera::Value::Number(n) => n.as_i64().unwrap_or(0),
        _ => return Ok(value.clone()),
    };

    if timestamp == 0 {
        return Ok(tera::Value::String("Never".to_string()));
    }

    let datetime = chrono::DateTime::from_timestamp(timestamp, 0)
        .unwrap_or_else(|| chrono::DateTime::from_timestamp(0, 0).unwrap());

    let formatted = datetime.format("%B %e, %Y - %l:%M%P").to_string();
    Ok(tera::Value::String(formatted))
}

#[derive(Clone)]
struct AppState {
    pool: MySqlPool,
    tera: Tera,
    config: Arc<Config>,
}

impl axum::extract::FromRef<AppState> for MySqlPool {
    fn from_ref(state: &AppState) -> Self {
        state.pool.clone()
    }
}

impl axum::extract::FromRef<AppState> for Tera {
    fn from_ref(state: &AppState) -> Self {
        state.tera.clone()
    }
}

impl axum::extract::FromRef<AppState> for Arc<Config> {
    fn from_ref(state: &AppState) -> Self {
        state.config.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Entering main");
    let mut tera = match Tera::new("templates/**/*.html") {
        Ok(t) => {
            println!("Tera initialized");
            t
        },
        Err(e) => {
            println!("Tera error: {:?}", e);
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };

    tera.register_filter("format_date", format_date_filter);

    // tracing_subscriber::fmt::init();

    dotenvy::dotenv().ok();

    let config = Config::from_env()?;
    println!("Starting server on {}", config.bind_address());
    println!("Using database URL: {}", config.database.url);

    let pool = db::create_pool(&config.database.url).await?;
    println!("Database connection established");

    let session_store = MySqlStore::new(pool.clone());
    println!("Migrating session store...");
    session_store.migrate().await?;
    use std::io::Write;
    println!("Session store migrated");
    
    println!("Creating SessionManagerLayer...");
    let session_layer = SessionManagerLayer::new(session_store);
    
    println!("SessionManagerLayer created. Adding expiry...");
    let session_layer = session_layer.with_expiry(Expiry::OnInactivity(time::Duration::days(7)));
    println!("Session layer created");

    let state = AppState {
        pool: pool.clone(),
        tera,
        config: Arc::new(config.clone()),
    };
    println!("AppState created");

    let app = Router::new()
        .route("/", get(handlers::home::index))
        .route("/install", get(handlers::install::welcome))
        .route("/install/database", get(handlers::install::database))
        .route("/install/admin", get(handlers::install::admin_form))
        .route("/install/admin", post(handlers::install::admin_submit))
        .route("/install/complete", get(handlers::install::complete))
        .route("/admin", get(handlers::admin::index))
        .route("/admin/node", get(handlers::admin::content_list))
        .route("/admin/node", post(handlers::admin::content_action))
        .route("/admin/node/types", get(handlers::admin::node_types))
        .route("/admin/node/types/:type", get(handlers::admin::node_type_edit_form))
        .route("/admin/node/types/:type", post(handlers::admin::node_type_edit_submit))
        .route("/admin/user", get(handlers::admin::user_list))
        .route("/admin/user", post(handlers::admin::user_action))
        .route("/admin/settings", get(handlers::admin::settings_form))
        .route("/admin/settings", post(handlers::admin::settings_submit))
        .route("/admin/reports/status", get(handlers::admin::status_report))
        .route("/user/login", get(handlers::user::login_form))
        .route("/user/login", post(handlers::user::login_submit))
        .route("/user/logout", get(handlers::user::logout))
        .route("/user/register", get(handlers::user::register_form))
        .route("/user/register", post(handlers::user::register_submit))
        .route("/user/:uid", get(handlers::user::profile))
        .route("/user/:uid/edit", get(handlers::user::edit_form))
        .route("/user/:uid/edit", post(handlers::user::edit_submit))
        .route("/node/add", get(handlers::node::list_types))
        .route("/node/add/:type", get(handlers::node::add_form))
        .route("/node/add/:type", post(handlers::node::add_submit))
        .route("/node/:nid", get(handlers::node::view))
        .route("/node/:nid/edit", get(handlers::node::edit_form))
        .route("/node/:nid/edit", post(handlers::node::edit_submit));

    println!("Base routes created");

    let app = app.nest_service("/static", ServeDir::new("static"));
    println!("Static routes added");

    let app = app.layer(middleware::from_fn_with_state(pool, auth_middleware));
    println!("Auth middleware added");

    let app = app.layer(session_layer);
    println!("Session middleware added");

    let app = app.with_state(state);
    println!("State added");

    println!("App router created");
    let listener = tokio::net::TcpListener::bind(config.bind_address()).await?;
    println!("Server listening on http://{}", config.bind_address());
    tracing::info!("Server listening on http://{}", config.bind_address());

    axum::serve(listener, app).await?;
    println!("Server stopped");

    Ok(())
}
