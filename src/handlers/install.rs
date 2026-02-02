use axum::{
    extract::State,
    response::{Html, Redirect},
    Form,
};
use serde::Deserialize;
use sqlx::MySqlPool;
use tera::Tera;

use crate::{
    auth::hash_password,
    db::migrations::{is_installed, run_migrations},
    error::{AppError, AppResult},
    models::User,
};

pub async fn welcome(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
) -> AppResult<Result<Html<String>, Redirect>> {
    if is_installed(&pool).await.unwrap_or(false) {
        return Ok(Err(Redirect::to("/")));
    }

    let mut context = tera::Context::new();
    context.insert("title", "Install Drupal");
    context.insert("step", "welcome");

    let html = tera.render("install/welcome.html", &context)?;
    Ok(Ok(Html(html)))
}

pub async fn database(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
) -> AppResult<Result<Html<String>, Redirect>> {
    if is_installed(&pool).await.unwrap_or(false) {
        return Ok(Err(Redirect::to("/")));
    }

    if let Err(e) = run_migrations(&pool).await {
        let mut context = tera::Context::new();
        context.insert("title", "Database Error");
        context.insert("error", &e.to_string());
        let html = tera.render("install/database.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    let mut context = tera::Context::new();
    context.insert("title", "Database Setup Complete");
    context.insert("success", &true);

    let html = tera.render("install/database.html", &context)?;
    Ok(Ok(Html(html)))
}

pub async fn admin_form(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
) -> AppResult<Result<Html<String>, Redirect>> {
    if is_installed(&pool).await.unwrap_or(false) {
        return Ok(Err(Redirect::to("/")));
    }

    let mut context = tera::Context::new();
    context.insert("title", "Create Admin Account");

    let html = tera.render("install/admin.html", &context)?;
    Ok(Ok(Html(html)))
}

#[derive(Debug, Deserialize)]
pub struct AdminForm {
    pub username: String,
    pub email: String,
    pub password: String,
    pub password_confirm: String,
}

pub async fn admin_submit(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Form(form): Form<AdminForm>,
) -> AppResult<Result<Html<String>, Redirect>> {
    if is_installed(&pool).await.unwrap_or(false) {
        return Ok(Err(Redirect::to("/")));
    }

    let mut context = tera::Context::new();
    context.insert("title", "Create Admin Account");

    if form.username.is_empty() {
        context.insert("error", "Username is required");
        let html = tera.render("install/admin.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    if form.password != form.password_confirm {
        context.insert("error", "Passwords do not match");
        let html = tera.render("install/admin.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    if form.password.len() < 6 {
        context.insert("error", "Password must be at least 6 characters");
        let html = tera.render("install/admin.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    let password_hash =
        hash_password(&form.password).map_err(|e| AppError::Internal(e.to_string()))?;

    let uid = User::create(&pool, &form.username, &password_hash, &form.email).await?;

    User::add_role(&pool, uid, 2).await?;
    User::add_role(&pool, uid, 3).await?;

    Ok(Err(Redirect::to("/install/complete")))
}

pub async fn complete(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
) -> AppResult<Html<String>> {
    if !is_installed(&pool).await.unwrap_or(false) {
        return Err(AppError::BadRequest(
            "Installation not complete".to_string(),
        ));
    }

    let mut context = tera::Context::new();
    context.insert("title", "Installation Complete");

    let html = tera.render("install/complete.html", &context)?;
    Ok(Html(html))
}
