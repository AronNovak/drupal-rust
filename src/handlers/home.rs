use axum::{extract::State, response::Html, Extension};
use sqlx::MySqlPool;
use tera::Tera;

use crate::{
    auth::middleware::CurrentUser,
    db::migrations::is_installed,
    error::AppResult,
    models::{Node, Variable},
};

pub async fn index(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
) -> AppResult<Html<String>> {
    let installed = is_installed(&pool).await.unwrap_or(false);

    let nodes = if installed {
        Node::find_promoted(&pool, 10).await?
    } else {
        vec![]
    };

    let site_name = Variable::get_or_default(&pool, "site_name", "Drupal").await;

    let mut context = tera::Context::new();
    context.insert("title", "Home");
    context.insert("nodes", &nodes);
    context.insert("current_user", &current_user);
    context.insert("installed", &installed);
    context.insert("site_name", &site_name);

    let html = tera.render("home.html", &context)?;
    Ok(Html(html))
}
