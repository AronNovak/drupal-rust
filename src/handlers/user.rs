use axum::{
    extract::{Path, Query, State},
    response::{Html, Redirect},
    Extension, Form,
};
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use std::collections::HashMap;
use tera::Tera;
use tower_sessions::Session;

use crate::{
    auth::{hash_password, middleware::CurrentUser, verify_password},
    error::{AppError, AppResult},
    models::{get_default_theme, session::SESSION_USER_KEY, ProfileField, ProfileValue, User},
};

#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    pub registered: Option<String>,
}

pub async fn login_form(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Query(query): Query<LoginQuery>,
) -> AppResult<Result<Html<String>, Redirect>> {
    if current_user.is_some() {
        return Ok(Err(Redirect::to("/")));
    }

    let current_theme = get_default_theme(&pool).await;
    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", "Log in");
    context.insert("registered", &query.registered.is_some());

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
    let current_theme = get_default_theme(&pool).await;
    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
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

pub async fn register_form(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
) -> AppResult<Result<Html<String>, Redirect>> {
    if current_user.is_some() {
        return Ok(Err(Redirect::to("/")));
    }

    let profile_fields = ProfileField::for_registration(&pool).await?;
    let current_theme = get_default_theme(&pool).await;

    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", "Create new account");
    context.insert("profile_fields", &profile_fields);

    let html = tera.render("user/register.html", &context)?;
    Ok(Ok(Html(html)))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterForm {
    pub username: String,
    pub email: String,
    pub password: String,
    pub password_confirm: String,
    #[serde(flatten)]
    pub profile: HashMap<String, String>,
}

pub async fn register_submit(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Form(form): Form<RegisterForm>,
) -> AppResult<Result<Html<String>, Redirect>> {
    if current_user.is_some() {
        return Ok(Err(Redirect::to("/")));
    }

    let profile_fields = ProfileField::for_registration(&pool).await?;
    let current_theme = get_default_theme(&pool).await;

    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", "Create new account");
    context.insert("profile_fields", &profile_fields);
    context.insert("form", &form);

    if form.username.is_empty() {
        context.insert("error", "Username is required");
        let html = tera.render("user/register.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    if form.username.len() < 3 {
        context.insert("error", "Username must be at least 3 characters");
        let html = tera.render("user/register.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    if !form
        .username
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        context.insert(
            "error",
            "Username may only contain letters, numbers, underscores, and hyphens",
        );
        let html = tera.render("user/register.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    if form.email.is_empty() || !form.email.contains('@') {
        context.insert("error", "Valid email address is required");
        let html = tera.render("user/register.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    if form.password.len() < 6 {
        context.insert("error", "Password must be at least 6 characters");
        let html = tera.render("user/register.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    if form.password != form.password_confirm {
        context.insert("error", "Passwords do not match");
        let html = tera.render("user/register.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    if User::find_by_name(&pool, &form.username).await?.is_some() {
        context.insert("error", "Username is already taken");
        let html = tera.render("user/register.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    if User::find_by_mail(&pool, &form.email).await?.is_some() {
        context.insert("error", "Email address is already registered");
        let html = tera.render("user/register.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    for field in &profile_fields {
        if field.required == 1 {
            let field_name = format!("profile_{}", field.fid);
            let value = form.profile.get(&field_name).map(|s| s.as_str()).unwrap_or("");
            if value.is_empty() {
                context.insert(
                    "error",
                    &format!("{} is required", field.title.as_deref().unwrap_or(&field.name)),
                );
                let html = tera.render("user/register.html", &context)?;
                return Ok(Ok(Html(html)));
            }
        }
    }

    let password_hash =
        hash_password(&form.password).map_err(|e| AppError::Internal(e.to_string()))?;

    let uid = User::create(&pool, &form.username, &password_hash, &form.email).await?;

    User::add_role(&pool, uid, 2).await?;

    for field in &profile_fields {
        let field_name = format!("profile_{}", field.fid);
        if let Some(value) = form.profile.get(&field_name) {
            if !value.is_empty() {
                ProfileValue::set(&pool, field.fid, uid, value).await?;
            }
        }
    }

    Ok(Err(Redirect::to("/user/login?registered=1")))
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

    let viewer_uid = current_user.as_ref().map(|u| u.uid);
    let profile_values = ProfileValue::get_visible_for_user(&pool, uid, viewer_uid).await?;
    let current_theme = get_default_theme(&pool).await;

    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", &user.name);
    context.insert("profile_user", &user);
    context.insert("current_user", &current_user);
    context.insert("profile_values", &profile_values);

    let html = tera.render("user/profile.html", &context)?;
    Ok(Html(html))
}

pub async fn edit_form(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(uid): Path<u32>,
) -> AppResult<Html<String>> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    if user.uid != uid && user.uid != 1 {
        return Err(AppError::Forbidden);
    }

    let profile_user = User::find_by_uid(&pool, uid)
        .await?
        .ok_or(AppError::NotFound)?;

    let profile_values = ProfileValue::get_for_user(&pool, uid).await?;
    let current_theme = get_default_theme(&pool).await;

    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", &format!("Edit {}", profile_user.name));
    context.insert("profile_user", &profile_user);
    context.insert("current_user", &Some(user));
    context.insert("profile_values", &profile_values);

    let html = tera.render("user/edit.html", &context)?;
    Ok(Html(html))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EditForm {
    pub email: String,
    pub password: Option<String>,
    pub password_confirm: Option<String>,
    #[serde(flatten)]
    pub profile: HashMap<String, String>,
}

pub async fn edit_submit(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(uid): Path<u32>,
    Form(form): Form<EditForm>,
) -> AppResult<Result<Html<String>, Redirect>> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    if user.uid != uid && user.uid != 1 {
        return Err(AppError::Forbidden);
    }

    let profile_user = User::find_by_uid(&pool, uid)
        .await?
        .ok_or(AppError::NotFound)?;

    let profile_values = ProfileValue::get_for_user(&pool, uid).await?;
    let current_theme = get_default_theme(&pool).await;

    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", &format!("Edit {}", profile_user.name));
    context.insert("profile_user", &profile_user);
    context.insert("current_user", &Some(&user));
    context.insert("profile_values", &profile_values);
    context.insert("form", &form);

    if form.email.is_empty() || !form.email.contains('@') {
        context.insert("error", "Valid email address is required");
        let html = tera.render("user/edit.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    if let Some(existing) = User::find_by_mail(&pool, &form.email).await? {
        if existing.uid != uid {
            context.insert("error", "Email address is already in use");
            let html = tera.render("user/edit.html", &context)?;
            return Ok(Ok(Html(html)));
        }
    }

    let new_password = form.password.as_ref().filter(|p| !p.is_empty());
    if let Some(password) = new_password {
        if password.len() < 6 {
            context.insert("error", "Password must be at least 6 characters");
            let html = tera.render("user/edit.html", &context)?;
            return Ok(Ok(Html(html)));
        }

        let confirm = form.password_confirm.as_deref().unwrap_or("");
        if password != confirm {
            context.insert("error", "Passwords do not match");
            let html = tera.render("user/edit.html", &context)?;
            return Ok(Ok(Html(html)));
        }
    }

    let all_fields = ProfileField::all(&pool).await?;
    for field in &all_fields {
        if field.required == 1 {
            let field_name = format!("profile_{}", field.fid);
            let value = form.profile.get(&field_name).map(|s| s.as_str()).unwrap_or("");
            if value.is_empty() {
                context.insert(
                    "error",
                    &format!("{} is required", field.title.as_deref().unwrap_or(&field.name)),
                );
                let html = tera.render("user/edit.html", &context)?;
                return Ok(Ok(Html(html)));
            }
        }
    }

    if let Some(password) = new_password {
        let password_hash =
            hash_password(password).map_err(|e| AppError::Internal(e.to_string()))?;
        User::update_password(&pool, uid, &password_hash).await?;
    }

    User::update_mail(&pool, uid, &form.email).await?;

    for field in &all_fields {
        let field_name = format!("profile_{}", field.fid);
        let value = form.profile.get(&field_name).map(|s| s.as_str()).unwrap_or("");
        ProfileValue::set(&pool, field.fid, uid, value).await?;
    }

    Ok(Err(Redirect::to(&format!("/user/{}", uid))))
}
