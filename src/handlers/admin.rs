use axum::{
    extract::State,
    response::Html,
    Extension,
};
use sqlx::MySqlPool;
use tera::Tera;

use crate::{
    auth::middleware::CurrentUser,
    error::{AppError, AppResult},
    models::NodeType,
};

pub async fn index(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
) -> AppResult<Html<String>> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    if !user.has_permission(&pool, "administer nodes").await? {
         return Err(AppError::Forbidden);
    }

    let mut context = tera::Context::new();
    context.insert("title", "Administer");
    context.insert("current_user", &Some(user));

    let admin_blocks = vec![
        ("Content management", vec![
            ("Content", "/admin/node"),
            ("Content types", "/admin/node/types"),
        ]),
        ("User management", vec![
            ("Users", "/admin/user"),
        ]),
    ];
    context.insert("admin_blocks", &admin_blocks);

    let html = tera.render("admin/index.html", &context)?;
    Ok(Html(html))
}

pub async fn node_types(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
) -> AppResult<Html<String>> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    if !user.has_permission(&pool, "administer nodes").await? {
         return Err(AppError::Forbidden);
    }

    let types = NodeType::all(&pool).await?;

    let mut context = tera::Context::new();
    context.insert("title", "Content types");
    context.insert("current_user", &Some(user));
    context.insert("types", &types);

    let html = tera.render("admin/node_types.html", &context)?;
    Ok(Html(html))
}
