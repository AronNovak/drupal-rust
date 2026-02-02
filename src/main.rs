mod auth;
mod config;
mod db;
mod error;
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
    tracing_subscriber::fmt::init();

    dotenvy::dotenv().ok();

    let config = Config::from_env()?;
    tracing::info!("Starting server on {}", config.bind_address());

    let pool = db::create_pool(&config.database.url).await?;
    tracing::info!("Database connection established");

    let session_store = MySqlStore::new(pool.clone());
    session_store.migrate().await?;

    let session_layer = SessionManagerLayer::new(session_store)
        .with_expiry(Expiry::OnInactivity(time::Duration::days(7)));

    let tera = Tera::new("templates/**/*.html")?;

    let state = AppState {
        pool: pool.clone(),
        tera,
        config: Arc::new(config.clone()),
    };

    let app = Router::new()
        .route("/", get(handlers::home::index))
        .route("/install", get(handlers::install::welcome))
        .route("/install/database", get(handlers::install::database))
        .route("/install/admin", get(handlers::install::admin_form))
        .route("/install/admin", post(handlers::install::admin_submit))
        .route("/install/complete", get(handlers::install::complete))
        .route("/user/login", get(handlers::user::login_form))
        .route("/user/login", post(handlers::user::login_submit))
        .route("/user/logout", get(handlers::user::logout))
        .route("/user/register", get(handlers::user::register_form))
        .route("/user/register", post(handlers::user::register_submit))
        .route("/user/{uid}", get(handlers::user::profile))
        .route("/user/{uid}/edit", get(handlers::user::edit_form))
        .route("/user/{uid}/edit", post(handlers::user::edit_submit))
        .route("/node/add", get(handlers::node::list_types))
        .route("/node/add/{type}", get(handlers::node::add_form))
        .route("/node/add/{type}", post(handlers::node::add_submit))
        .route("/node/{nid}", get(handlers::node::view))
        .route("/node/{nid}/edit", get(handlers::node::edit_form))
        .route("/node/{nid}/edit", post(handlers::node::edit_submit))
        .nest_service("/static", ServeDir::new("static"))
        .layer(middleware::from_fn_with_state(pool, auth_middleware))
        .layer(session_layer)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.bind_address()).await?;
    tracing::info!("Server listening on http://{}", config.bind_address());

    axum::serve(listener, app).await?;

    Ok(())
}
