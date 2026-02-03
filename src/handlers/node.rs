use axum::{
    extract::{Path, State},
    response::{Html, Redirect},
    Extension, Form,
};
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use std::collections::HashMap;
use tera::Tera;

use crate::{
    auth::middleware::CurrentUser,
    error::{AppError, AppResult},
    models::{get_default_theme, get_fields_with_values, save_field_values, Comment, Node, NodeFieldInstance, NodeType, COMMENT_NODE_DISABLED},
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

    let fields = get_fields_with_values(&pool, &node.node_type, node.vid).await?;
    let current_theme = get_default_theme(&pool).await;

    // Load comments if enabled
    let comments = if node.comment != COMMENT_NODE_DISABLED {
        let is_admin = current_user.as_ref().map(|u| u.uid == 1).unwrap_or(false);
        Comment::find_for_node(&pool, nid, is_admin).await?
    } else {
        vec![]
    };

    // Check comment permissions
    let can_post_comments = check_post_comment_permission(&pool, &current_user).await?;
    let can_administer_comments = match &current_user {
        Some(user) => user.has_permission(&pool, "administer comments").await?,
        None => false,
    };

    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", &node.title);
    context.insert("node", &node);
    context.insert("fields", &fields);
    context.insert("current_user", &current_user);
    context.insert("comments", &comments);
    context.insert("can_post_comments", &can_post_comments);
    context.insert("can_administer_comments", &can_administer_comments);

    let html = tera.render("node/view.html", &context)?;
    Ok(Html(html))
}

async fn check_post_comment_permission(
    pool: &MySqlPool,
    current_user: &Option<crate::models::User>,
) -> Result<bool, sqlx::Error> {
    match current_user {
        Some(user) => user.has_permission(pool, "post comments").await,
        None => {
            let result: Option<(String,)> =
                sqlx::query_as("SELECT perm FROM permission WHERE rid = 1")
                    .fetch_optional(pool)
                    .await?;
            Ok(result
                .map(|(perm,)| perm.contains("post comments"))
                .unwrap_or(false))
        }
    }
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

    let fields = NodeFieldInstance::with_field_info(&pool, &node_type).await?;
    let current_theme = get_default_theme(&pool).await;

    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", &format!("Create {}", type_info.name));
    context.insert("node_type", &type_info);
    context.insert("fields", &fields);
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
    #[serde(flatten)]
    pub field_values: HashMap<String, String>,
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

    let fields = NodeFieldInstance::with_field_info(&pool, &node_type).await?;
    let current_theme = get_default_theme(&pool).await;

    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", &format!("Create {}", type_info.name));
    context.insert("node_type", &type_info);
    context.insert("fields", &fields);
    context.insert("current_user", &Some(&user));
    context.insert("form", &form);

    if form.title.is_empty() {
        context.insert("error", "Title is required");
        let html = tera.render("node/form.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    for field in &fields {
        if field.required == 1 {
            let key = format!("field_{}", field.field_name);
            let value = form.field_values.get(&key).map(|s| s.as_str()).unwrap_or("");
            if value.is_empty() {
                context.insert("error", &format!("{} is required", field.label));
                let html = tera.render("node/form.html", &context)?;
                return Ok(Ok(Html(html)));
            }
        }
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

    let (nid, vid) = Node::create(
        &pool,
        &node_type,
        &form.title,
        &form.body,
        &teaser,
        user.uid,
        promote,
        sticky,
    )
    .await?;

    save_field_values(&pool, nid, vid, &node_type, &form.field_values).await?;

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

    let fields = get_fields_with_values(&pool, &node.node_type, node.vid).await?;
    let current_theme = get_default_theme(&pool).await;

    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", &format!("Edit {}", node.title));
    context.insert("node", &node);
    context.insert("node_type", &type_info);
    context.insert("fields", &fields);
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

    let type_info = NodeType::find_by_type(&pool, &node.node_type)
        .await?
        .ok_or(AppError::NotFound)?;

    let fields = get_fields_with_values(&pool, &node.node_type, node.vid).await?;
    let current_theme = get_default_theme(&pool).await;

    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", &format!("Edit {}", node.title));
    context.insert("node", &node);
    context.insert("node_type", &type_info);
    context.insert("fields", &fields);
    context.insert("current_user", &Some(&user));
    context.insert("editing", &true);
    context.insert("form", &form);

    if form.title.is_empty() {
        context.insert("error", "Title is required");
        let html = tera.render("node/form.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    for field in &fields {
        if field.required == 1 {
            let key = format!("field_{}", field.field_name);
            let value = form.field_values.get(&key).map(|s| s.as_str()).unwrap_or("");
            if value.is_empty() {
                context.insert("error", &format!("{} is required", field.label));
                let html = tera.render("node/form.html", &context)?;
                return Ok(Ok(Html(html)));
            }
        }
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

    let vid = Node::update(
        &pool,
        nid,
        &form.title,
        &form.body,
        &teaser,
        user.uid,
        promote,
        sticky,
    )
    .await?;

    save_field_values(&pool, nid, vid, &node.node_type, &form.field_values).await?;

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
    let current_theme = get_default_theme(&pool).await;

    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", "Add content");
    context.insert("types", &types);
    context.insert("current_user", &Some(user));

    let html = tera.render("node/list.html", &context)?;
    Ok(Html(html))
}
