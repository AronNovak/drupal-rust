use axum::{
    extract::{ConnectInfo, Path, State},
    response::{Html, Redirect},
    Extension, Form,
};
use serde::{Deserialize, Serialize};
use sqlx::MySqlPool;
use std::net::SocketAddr;
use tera::Tera;

use crate::{
    auth::middleware::CurrentUser,
    error::{AppError, AppResult},
    models::{
        get_default_theme, Comment, Node, COMMENT_NODE_DISABLED, COMMENT_NODE_READ_WRITE,
        COMMENT_NOT_PUBLISHED, COMMENT_PUBLISHED,
    },
};

#[derive(Debug, Deserialize, Serialize)]
pub struct CommentForm {
    pub subject: String,
    pub comment: String,
    pub name: Option<String>,
    pub mail: Option<String>,
    pub homepage: Option<String>,
}

/// GET /comment/reply/:nid - Show comment form for a node
pub async fn add_form(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(nid): Path<u32>,
) -> AppResult<Html<String>> {
    let node = Node::find_with_body(&pool, nid)
        .await?
        .ok_or(AppError::NotFound)?;

    if node.comment == COMMENT_NODE_DISABLED {
        return Err(AppError::Forbidden);
    }

    // Check permission
    let can_post = check_post_permission(&pool, &current_user).await?;
    if !can_post {
        return Err(AppError::Forbidden);
    }

    let current_theme = get_default_theme(&pool).await;
    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", &format!("Reply to {}", node.title));
    context.insert("node", &node);
    context.insert("current_user", &current_user);
    context.insert("pid", &0u32);

    let html = tera.render("comment/form.html", &context)?;
    Ok(Html(html))
}

/// POST /comment/reply/:nid - Submit a new comment
pub async fn add_submit(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(nid): Path<u32>,
    Form(form): Form<CommentForm>,
) -> AppResult<Result<Html<String>, Redirect>> {
    let node = Node::find_with_body(&pool, nid)
        .await?
        .ok_or(AppError::NotFound)?;

    if node.comment != COMMENT_NODE_READ_WRITE {
        return Err(AppError::Forbidden);
    }

    let can_post = check_post_permission(&pool, &current_user).await?;
    if !can_post {
        return Err(AppError::Forbidden);
    }

    let current_theme = get_default_theme(&pool).await;
    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", &format!("Reply to {}", node.title));
    context.insert("node", &node);
    context.insert("current_user", &current_user);
    context.insert("form", &form);
    context.insert("pid", &0u32);

    // Validation
    if form.comment.trim().is_empty() {
        context.insert("error", "Comment body is required");
        let html = tera.render("comment/form.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    // For anonymous users, name is required
    if current_user.is_none() && form.name.as_ref().map(|n| n.trim().is_empty()).unwrap_or(true) {
        context.insert("error", "Your name is required");
        let html = tera.render("comment/form.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    let uid = current_user.as_ref().map(|u| u.uid).unwrap_or(0);
    let hostname = addr.ip().to_string();

    // Use subject if provided, otherwise generate from comment
    let subject = if form.subject.trim().is_empty() {
        truncate_subject(&form.comment)
    } else {
        form.subject.clone()
    };

    // Check if user can post without approval
    let status = if check_post_without_approval(&pool, &current_user).await? {
        COMMENT_PUBLISHED
    } else {
        COMMENT_NOT_PUBLISHED
    };

    let cid = Comment::create(
        &pool,
        nid,
        0, // pid = 0 for top-level comments
        uid,
        &subject,
        &form.comment,
        &hostname,
        form.name.as_deref(),
        form.mail.as_deref(),
        form.homepage.as_deref(),
        status,
    )
    .await?;

    Ok(Err(Redirect::to(&format!("/node/{}#comment-{}", nid, cid))))
}

/// GET /comment/reply/:cid/reply - Show reply form for a comment
pub async fn reply_form(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(cid): Path<u32>,
) -> AppResult<Html<String>> {
    let parent = Comment::find_by_cid(&pool, cid)
        .await?
        .ok_or(AppError::NotFound)?;

    let node = Node::find_with_body(&pool, parent.nid)
        .await?
        .ok_or(AppError::NotFound)?;

    if node.comment != COMMENT_NODE_READ_WRITE {
        return Err(AppError::Forbidden);
    }

    let can_post = check_post_permission(&pool, &current_user).await?;
    if !can_post {
        return Err(AppError::Forbidden);
    }

    let current_theme = get_default_theme(&pool).await;
    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", &format!("Reply to comment"));
    context.insert("node", &node);
    context.insert("parent", &parent);
    context.insert("current_user", &current_user);
    context.insert("pid", &cid);

    let html = tera.render("comment/form.html", &context)?;
    Ok(Html(html))
}

/// POST /comment/reply/:cid/reply - Submit a reply to a comment
pub async fn reply_submit(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Path(cid): Path<u32>,
    Form(form): Form<CommentForm>,
) -> AppResult<Result<Html<String>, Redirect>> {
    let parent = Comment::find_by_cid(&pool, cid)
        .await?
        .ok_or(AppError::NotFound)?;

    let node = Node::find_with_body(&pool, parent.nid)
        .await?
        .ok_or(AppError::NotFound)?;

    if node.comment != COMMENT_NODE_READ_WRITE {
        return Err(AppError::Forbidden);
    }

    let can_post = check_post_permission(&pool, &current_user).await?;
    if !can_post {
        return Err(AppError::Forbidden);
    }

    let current_theme = get_default_theme(&pool).await;
    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", "Reply to comment");
    context.insert("node", &node);
    context.insert("parent", &parent);
    context.insert("current_user", &current_user);
    context.insert("form", &form);
    context.insert("pid", &cid);

    // Validation
    if form.comment.trim().is_empty() {
        context.insert("error", "Comment body is required");
        let html = tera.render("comment/form.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    if current_user.is_none() && form.name.as_ref().map(|n| n.trim().is_empty()).unwrap_or(true) {
        context.insert("error", "Your name is required");
        let html = tera.render("comment/form.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    let uid = current_user.as_ref().map(|u| u.uid).unwrap_or(0);
    let hostname = addr.ip().to_string();

    let subject = if form.subject.trim().is_empty() {
        truncate_subject(&form.comment)
    } else {
        form.subject.clone()
    };

    let status = if check_post_without_approval(&pool, &current_user).await? {
        COMMENT_PUBLISHED
    } else {
        COMMENT_NOT_PUBLISHED
    };

    let new_cid = Comment::create(
        &pool,
        parent.nid,
        cid, // pid = parent comment
        uid,
        &subject,
        &form.comment,
        &hostname,
        form.name.as_deref(),
        form.mail.as_deref(),
        form.homepage.as_deref(),
        status,
    )
    .await?;

    Ok(Err(Redirect::to(&format!(
        "/node/{}#comment-{}",
        parent.nid, new_cid
    ))))
}

/// GET /comment/:cid/edit - Show edit form
pub async fn edit_form(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(cid): Path<u32>,
) -> AppResult<Html<String>> {
    let comment = Comment::find_by_cid(&pool, cid)
        .await?
        .ok_or(AppError::NotFound)?;

    let can_edit = check_edit_permission(&pool, &current_user, &comment).await?;
    if !can_edit {
        return Err(AppError::Forbidden);
    }

    let node = Node::find_with_body(&pool, comment.nid)
        .await?
        .ok_or(AppError::NotFound)?;

    let current_theme = get_default_theme(&pool).await;
    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", "Edit comment");
    context.insert("node", &node);
    context.insert("comment", &comment);
    context.insert("current_user", &current_user);
    context.insert("editing", &true);

    // Pre-fill form
    let form = CommentForm {
        subject: comment.subject.clone(),
        comment: comment.comment.clone(),
        name: comment.name.clone(),
        mail: comment.mail.clone(),
        homepage: comment.homepage.clone(),
    };
    context.insert("form", &form);

    let html = tera.render("comment/form.html", &context)?;
    Ok(Html(html))
}

/// POST /comment/:cid/edit - Submit edit
pub async fn edit_submit(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(cid): Path<u32>,
    Form(form): Form<CommentForm>,
) -> AppResult<Result<Html<String>, Redirect>> {
    let comment = Comment::find_by_cid(&pool, cid)
        .await?
        .ok_or(AppError::NotFound)?;

    let can_edit = check_edit_permission(&pool, &current_user, &comment).await?;
    if !can_edit {
        return Err(AppError::Forbidden);
    }

    let node = Node::find_with_body(&pool, comment.nid)
        .await?
        .ok_or(AppError::NotFound)?;

    let current_theme = get_default_theme(&pool).await;
    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", "Edit comment");
    context.insert("node", &node);
    context.insert("comment", &comment);
    context.insert("current_user", &current_user);
    context.insert("form", &form);
    context.insert("editing", &true);

    if form.comment.trim().is_empty() {
        context.insert("error", "Comment body is required");
        let html = tera.render("comment/form.html", &context)?;
        return Ok(Ok(Html(html)));
    }

    let subject = if form.subject.trim().is_empty() {
        truncate_subject(&form.comment)
    } else {
        form.subject.clone()
    };

    Comment::update(&pool, cid, &subject, &form.comment, comment.status).await?;

    Ok(Err(Redirect::to(&format!(
        "/node/{}#comment-{}",
        comment.nid, cid
    ))))
}

/// GET /comment/:cid/delete - Show delete confirmation
pub async fn delete_confirm(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(cid): Path<u32>,
) -> AppResult<Html<String>> {
    let comment = Comment::find_by_cid(&pool, cid)
        .await?
        .ok_or(AppError::NotFound)?;

    let can_delete = check_delete_permission(&pool, &current_user, &comment).await?;
    if !can_delete {
        return Err(AppError::Forbidden);
    }

    let node = Node::find_with_body(&pool, comment.nid)
        .await?
        .ok_or(AppError::NotFound)?;

    let current_theme = get_default_theme(&pool).await;
    let mut context = tera::Context::new();
    context.insert("current_theme", &current_theme);
    context.insert("title", "Delete comment");
    context.insert("node", &node);
    context.insert("comment", &comment);
    context.insert("current_user", &current_user);

    let html = tera.render("comment/delete.html", &context)?;
    Ok(Html(html))
}

/// POST /comment/:cid/delete - Execute delete
pub async fn delete_submit(
    State(pool): State<MySqlPool>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(cid): Path<u32>,
) -> AppResult<Redirect> {
    let comment = Comment::find_by_cid(&pool, cid)
        .await?
        .ok_or(AppError::NotFound)?;

    let can_delete = check_delete_permission(&pool, &current_user, &comment).await?;
    if !can_delete {
        return Err(AppError::Forbidden);
    }

    let nid = comment.nid;
    Comment::delete(&pool, cid).await?;

    Ok(Redirect::to(&format!("/node/{}", nid)))
}

// Helper functions

async fn check_post_permission(
    pool: &MySqlPool,
    current_user: &Option<crate::models::User>,
) -> Result<bool, sqlx::Error> {
    match current_user {
        Some(user) => user.has_permission(pool, "post comments").await,
        None => {
            // Check if anonymous role has post comments permission
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

async fn check_post_without_approval(
    pool: &MySqlPool,
    current_user: &Option<crate::models::User>,
) -> Result<bool, sqlx::Error> {
    match current_user {
        Some(user) => {
            // Admins always post without approval
            if user.uid == 1 {
                return Ok(true);
            }
            user.has_permission(pool, "post comments without approval")
                .await
        }
        None => Ok(false),
    }
}

async fn check_edit_permission(
    pool: &MySqlPool,
    current_user: &Option<crate::models::User>,
    comment: &Comment,
) -> Result<bool, sqlx::Error> {
    let Some(user) = current_user else {
        return Ok(false);
    };

    // Admin can always edit
    if user.has_permission(pool, "administer comments").await? {
        return Ok(true);
    }

    // Owner can edit own comments
    if user.uid == comment.uid && comment.uid != 0 {
        return Ok(true);
    }

    Ok(false)
}

async fn check_delete_permission(
    pool: &MySqlPool,
    current_user: &Option<crate::models::User>,
    _comment: &Comment,
) -> Result<bool, sqlx::Error> {
    let Some(user) = current_user else {
        return Ok(false);
    };

    // Only admins can delete
    user.has_permission(pool, "administer comments").await
}

fn truncate_subject(text: &str) -> String {
    let clean = text.trim();
    if clean.len() <= 60 {
        clean.to_string()
    } else {
        format!("{}...", &clean[..57])
    }
}
