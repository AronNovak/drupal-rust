use axum::{
    extract::{Path, State},
    response::{Html, Redirect},
    Extension, Form,
};
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use tera::Tera;

use crate::{
    auth::middleware::CurrentUser,
    error::{AppError, AppResult},
    models::{Node, NodeType},
};

pub async fn view(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(nid): Path<u32>,
) -> AppResult<Html<String>> {
    let node = Node::find_with_body(&pool, nid)
        .await?
        .ok_or(AppError::NotFound)?;

    if node.status != 1 {
        let can_view = current_user
            .as_ref()
            .map(|u| u.uid == node.uid || u.uid == 1)
            .unwrap_or(false);

        if !can_view {
            return Err(AppError::NotFound);
        }
    }

    let mut context = tera::Context::new();
    context.insert("title", &node.title);
    context.insert("node", &node);
    context.insert("current_user", &current_user);

    let html = tera.render("node/view.html", &context)?;
    Ok(Html(html))
}

pub async fn add_form(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(node_type): Path<String>,
) -> AppResult<Html<String>> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    let type_info = NodeType::find_by_type(&pool, &node_type)
        .await?
        .ok_or(AppError::NotFound)?;

    let mut context = tera::Context::new();
    context.insert("title", &format!("Create {}", type_info.name));
    context.insert("node_type", &type_info);
    context.insert("current_user", &Some(user));

    let html = tera.render("node/form.html", &context)?;
    Ok(Html(html))
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NodeForm {
    pub title: String,
    pub body: String,
    pub promote: Option<String>,
    pub sticky: Option<String>,
}

pub async fn add_submit(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(node_type): Path<String>,
    Form(form): Form<NodeForm>,
) -> AppResult<Result<Html<String>, Redirect>> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    let type_info = NodeType::find_by_type(&pool, &node_type)
        .await?
        .ok_or(AppError::NotFound)?;

    if form.title.is_empty() {
        let mut context = tera::Context::new();
        context.insert("title", &format!("Create {}", type_info.name));
        context.insert("node_type", &type_info);
        context.insert("current_user", &Some(&user));
        context.insert("error", "Title is required");
        context.insert("form", &form);

        let html = tera.render("node/form.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    let teaser = form
        .body
        .chars()
        .take(600)
        .collect::<String>()
        .split("\n\n")
        .next()
        .unwrap_or("")
        .to_string();

    let promote = form.promote.is_some();
    let sticky = form.sticky.is_some();

    let nid =
        Node::create(&pool, &node_type, &form.title, &form.body, &teaser, user.uid, promote, sticky)
            .await?;

    Ok(Err(Redirect::to(&format!("/node/{}", nid))))
}

pub async fn edit_form(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(nid): Path<u32>,
) -> AppResult<Html<String>> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    let node = Node::find_with_body(&pool, nid)
        .await?
        .ok_or(AppError::NotFound)?;

    let can_edit = user.uid == node.uid || user.uid == 1;
    if !can_edit {
        return Err(AppError::Forbidden);
    }

    let type_info = NodeType::find_by_type(&pool, &node.node_type)
        .await?
        .ok_or(AppError::NotFound)?;

    let mut context = tera::Context::new();
    context.insert("title", &format!("Edit {}", node.title));
    context.insert("node", &node);
    context.insert("node_type", &type_info);
    context.insert("current_user", &Some(user));
    context.insert("editing", &true);

    let html = tera.render("node/form.html", &context)?;
    Ok(Html(html))
}

pub async fn edit_submit(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(nid): Path<u32>,
    Form(form): Form<NodeForm>,
) -> AppResult<Result<Html<String>, Redirect>> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    let node = Node::find_with_body(&pool, nid)
        .await?
        .ok_or(AppError::NotFound)?;

    let can_edit = user.uid == node.uid || user.uid == 1;
    if !can_edit {
        return Err(AppError::Forbidden);
    }

    if form.title.is_empty() {
        let type_info = NodeType::find_by_type(&pool, &node.node_type)
            .await?
            .ok_or(AppError::NotFound)?;

        let mut context = tera::Context::new();
        context.insert("title", &format!("Edit {}", node.title));
        context.insert("node", &node);
        context.insert("node_type", &type_info);
        context.insert("current_user", &Some(&user));
        context.insert("editing", &true);
        context.insert("error", "Title is required");

        let html = tera.render("node/form.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    let teaser = form
        .body
        .chars()
        .take(600)
        .collect::<String>()
        .split("\n\n")
        .next()
        .unwrap_or("")
        .to_string();

    let promote = form.promote.is_some();
    let sticky = form.sticky.is_some();

    Node::update(&pool, nid, &form.title, &form.body, &teaser, user.uid, promote, sticky).await?;

    Ok(Err(Redirect::to(&format!("/node/{}", nid))))
}

pub async fn list_types(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
) -> AppResult<Html<String>> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    let types = NodeType::all(&pool).await?;

    let mut context = tera::Context::new();
    context.insert("title", "Add content");
    context.insert("types", &types);
    context.insert("current_user", &Some(user));

    let html = tera.render("node/list.html", &context)?;
    Ok(Html(html))
}
