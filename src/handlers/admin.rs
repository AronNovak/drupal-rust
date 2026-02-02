use axum::{
    extract::{Path, State},
    response::{Html, Redirect},
    Extension, Form,
};
use crate::extractors::QsForm;
use serde::Deserialize;
use sqlx::MySqlPool;
use tera::Tera;

use crate::{
    auth::middleware::CurrentUser,
    error::{AppError, AppResult},
    models::{Node, NodeType, User, Variable},
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
        ("Site configuration", vec![
            ("Site information", "/admin/settings"),
        ]),
        ("Reports", vec![
            ("Status report", "/admin/reports/status"),
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

pub async fn content_list(
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

    let nodes = Node::all_for_admin(&pool).await?;

    let mut context = tera::Context::new();
    context.insert("title", "Content");
    context.insert("current_user", &Some(user));
    context.insert("nodes", &nodes);

    let html = tera.render("admin/content.html", &context)?;
    Ok(Html(html))
}

pub async fn user_list(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
) -> AppResult<Html<String>> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    if !user.has_permission(&pool, "administer users").await? {
        return Err(AppError::Forbidden);
    }

    let users = User::all(&pool).await?;

    let mut context = tera::Context::new();
    context.insert("title", "Users");
    context.insert("current_user", &Some(user));
    context.insert("users", &users);

    let html = tera.render("admin/users.html", &context)?;
    Ok(Html(html))
}

pub async fn node_type_edit_form(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(type_name): Path<String>,
) -> AppResult<Html<String>> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    if !user.has_permission(&pool, "administer nodes").await? {
        return Err(AppError::Forbidden);
    }

    let Some(node_type) = NodeType::find_by_type(&pool, &type_name).await? else {
        return Err(AppError::NotFound);
    };

    let mut context = tera::Context::new();
    context.insert("title", &format!("Edit {}", node_type.name));
    context.insert("current_user", &Some(user));
    context.insert("node_type", &node_type);

    let html = tera.render("admin/node_type_edit.html", &context)?;
    Ok(Html(html))
}

#[derive(Debug, Deserialize)]
pub struct NodeTypeEditForm {
    pub name: String,
    pub description: String,
    pub help: String,
}

pub async fn node_type_edit_submit(
    State(pool): State<MySqlPool>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(type_name): Path<String>,
    Form(form): Form<NodeTypeEditForm>,
) -> AppResult<Redirect> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    if !user.has_permission(&pool, "administer nodes").await? {
        return Err(AppError::Forbidden);
    }

    NodeType::update(&pool, &type_name, &form.name, &form.description, &form.help).await?;

    Ok(Redirect::to("/admin/node/types"))
}

#[derive(Debug, Deserialize)]
pub struct ContentActionForm {
    pub action: String,
    #[serde(default)]
    pub nids: Vec<u32>,
}

pub async fn content_action(
    State(pool): State<MySqlPool>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    QsForm(form): QsForm<ContentActionForm>,
) -> AppResult<Redirect> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    if !user.has_permission(&pool, "administer nodes").await? {
        return Err(AppError::Forbidden);
    }

    for nid in form.nids {
        match form.action.as_str() {
            "publish" => Node::set_status(&pool, nid, 1).await?,
            "unpublish" => Node::set_status(&pool, nid, 0).await?,
            "delete" => Node::delete(&pool, nid).await?,
            _ => {}
        }
    }

    Ok(Redirect::to("/admin/node"))
}

#[derive(Debug, Deserialize)]
pub struct UserActionForm {
    pub action: String,
    #[serde(default)]
    pub uids: Vec<u32>,
}

pub async fn user_action(
    State(pool): State<MySqlPool>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    QsForm(form): QsForm<UserActionForm>,
) -> AppResult<Redirect> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    if !user.has_permission(&pool, "administer users").await? {
        return Err(AppError::Forbidden);
    }

    for uid in form.uids {
        if uid == 1 {
            continue;
        }
        match form.action.as_str() {
            "block" => User::set_status(&pool, uid, 0).await?,
            "unblock" => User::set_status(&pool, uid, 1).await?,
            _ => {}
        }
    }

    Ok(Redirect::to("/admin/user"))
}

pub async fn settings_form(
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

    let site_name = Variable::get_or_default(&pool, "site_name", "Drupal").await;
    let site_slogan = Variable::get_or_default(&pool, "site_slogan", "").await;
    let site_mail = Variable::get_or_default(&pool, "site_mail", "").await;
    let site_footer = Variable::get_or_default(&pool, "site_footer", "").await;

    let mut context = tera::Context::new();
    context.insert("title", "Site information");
    context.insert("current_user", &Some(user));
    context.insert("site_name", &site_name);
    context.insert("site_slogan", &site_slogan);
    context.insert("site_mail", &site_mail);
    context.insert("site_footer", &site_footer);

    let html = tera.render("admin/settings.html", &context)?;
    Ok(Html(html))
}

#[derive(Debug, Deserialize)]
pub struct SettingsForm {
    pub site_name: String,
    pub site_slogan: String,
    pub site_mail: String,
    pub site_footer: String,
}

pub async fn settings_submit(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Form(form): Form<SettingsForm>,
) -> AppResult<Html<String>> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    if !user.has_permission(&pool, "administer nodes").await? {
        return Err(AppError::Forbidden);
    }

    Variable::set(&pool, "site_name", &form.site_name).await?;
    Variable::set(&pool, "site_slogan", &form.site_slogan).await?;
    Variable::set(&pool, "site_mail", &form.site_mail).await?;
    Variable::set(&pool, "site_footer", &form.site_footer).await?;

    let mut context = tera::Context::new();
    context.insert("title", "Site information");
    context.insert("current_user", &Some(user));
    context.insert("site_name", &form.site_name);
    context.insert("site_slogan", &form.site_slogan);
    context.insert("site_mail", &form.site_mail);
    context.insert("site_footer", &form.site_footer);
    context.insert("message", "The configuration options have been saved.");

    let html = tera.render("admin/settings.html", &context)?;
    Ok(Html(html))
}

pub async fn status_report(
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

    let node_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM node")
        .fetch_one(&pool)
        .await?;
    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE uid > 0")
        .fetch_one(&pool)
        .await?;

    let mut context = tera::Context::new();
    context.insert("title", "Status report");
    context.insert("current_user", &Some(user));
    context.insert("drupal_version", "4.7.0-rust");
    context.insert("node_count", &node_count.0);
    context.insert("user_count", &user_count.0);

    let html = tera.render("admin/status.html", &context)?;
    Ok(Html(html))
}
