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
    println!("Entering main");
    let tera = match Tera::new("templates/**/*.html") {
        Ok(t) => {
            println!("Tera initialized");
            t
        },
        Err(e) => {
            println!("Tera error: {:?}", e);
            return Err(Box::new(e) as Box<dyn std::error::Error>);
        }
    };

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
        .route("/admin/node/types", get(handlers::admin::node_types))
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
