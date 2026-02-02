use axum::{
    extract::{Path, State},
    response::{Html, Redirect},
    Extension, Form,
};
use serde::Deserialize;
use sqlx::MySqlPool;
use tera::Tera;
use tower_sessions::Session;

use crate::{
    auth::{middleware::CurrentUser, verify_password},
    error::{AppError, AppResult},
    models::{session::SESSION_USER_KEY, User},
};

pub async fn login_form(
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
) -> AppResult<Result<Html<String>, Redirect>> {
    if current_user.is_some() {
        return Ok(Err(Redirect::to("/")));
    }

    let mut context = tera::Context::new();
    context.insert("title", "Log in");

    let html = tera.render("user/login.html", &context)?;
    Ok(Ok(Html(html)))
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

pub async fn login_submit(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    session: Session,
    Form(form): Form<LoginForm>,
) -> AppResult<Result<Html<String>, Redirect>> {
    let mut context = tera::Context::new();
    context.insert("title", "Log in");

    let Some(user) = User::find_by_name(&pool, &form.username).await? else {
        context.insert("error", "Invalid username or password");
        let html = tera.render("user/login.html", &context)?;
        return Ok(Ok(Html(html)));
    };

    if user.status != 1 {
        context.insert("error", "This account is blocked");
        let html = tera.render("user/login.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    if !verify_password(&form.password, &user.pass) {
        context.insert("error", "Invalid username or password");
        let html = tera.render("user/login.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    user.update_login(&pool).await?;

    session
        .insert(SESSION_USER_KEY, user.uid)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Err(Redirect::to("/")))
}

pub async fn logout(session: Session) -> AppResult<Redirect> {
    session
        .delete()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Redirect::to("/"))
}

pub async fn profile(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(uid): Path<u32>,
) -> AppResult<Html<String>> {
    let user = User::find_by_uid(&pool, uid)
        .await?
        .ok_or(AppError::NotFound)?;

    if user.status != 1 && current_user.as_ref().map(|u| u.uid).unwrap_or(0) != 1 {
        return Err(AppError::NotFound);
    }

    let mut context = tera::Context::new();
    context.insert("title", &user.name);
    context.insert("profile_user", &user);
    context.insert("current_user", &current_user);

    let html = tera.render("user/profile.html", &context)?;
    Ok(Html(html))
}
