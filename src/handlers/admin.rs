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
    models::{AccessLog, Node, NodeType, SystemItem, User, Variable},
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
        ("Site building", vec![
            ("Modules", "/admin/modules"),
            ("Themes", "/admin/themes"),
        ]),
        ("Site configuration", vec![
            ("Site information", "/admin/settings"),
        ]),
        ("Logs", vec![
            ("Recent hits", "/admin/logs/hits"),
            ("Top pages", "/admin/logs/pages"),
            ("Top visitors", "/admin/logs/visitors"),
            ("Top referrers", "/admin/logs/referrers"),
            ("Statistics settings", "/admin/logs/settings"),
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

// Module administration
pub async fn modules_list(
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

    let modules = SystemItem::all_modules(&pool).await?;

    let mut context = tera::Context::new();
    context.insert("title", "Modules");
    context.insert("current_user", &Some(user));
    context.insert("modules", &modules);

    let html = tera.render("admin/modules.html", &context)?;
    Ok(Html(html))
}

#[derive(Debug, Deserialize)]
pub struct ModulesForm {
    #[serde(default)]
    pub modules: Vec<String>,
}

pub async fn modules_submit(
    State(pool): State<MySqlPool>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    QsForm(form): QsForm<ModulesForm>,
) -> AppResult<Redirect> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    if !user.has_permission(&pool, "administer nodes").await? {
        return Err(AppError::Forbidden);
    }

    let all_modules = SystemItem::all_modules(&pool).await?;

    for module in &all_modules {
        if module.is_required_module() {
            continue;
        }

        let should_enable = form.modules.contains(&module.name);
        if should_enable && module.status == 0 {
            SystemItem::enable_module(&pool, &module.name).await?;
        } else if !should_enable && module.status == 1 {
            SystemItem::disable_module(&pool, &module.name).await?;
        }
    }

    Ok(Redirect::to("/admin/modules"))
}

// Theme administration
pub async fn themes_list(
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

    let themes = SystemItem::all_themes(&pool).await?;
    let default_theme = crate::models::get_default_theme(&pool).await;

    let mut context = tera::Context::new();
    context.insert("title", "Themes");
    context.insert("current_user", &Some(user));
    context.insert("themes", &themes);
    context.insert("default_theme", &default_theme);

    let html = tera.render("admin/themes.html", &context)?;
    Ok(Html(html))
}

#[derive(Debug, Deserialize)]
pub struct ThemesForm {
    pub default_theme: String,
    #[serde(default)]
    pub themes: Vec<String>,
}

pub async fn themes_submit(
    State(pool): State<MySqlPool>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    QsForm(form): QsForm<ThemesForm>,
) -> AppResult<Redirect> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    if !user.has_permission(&pool, "administer nodes").await? {
        return Err(AppError::Forbidden);
    }

    let all_themes = SystemItem::all_themes(&pool).await?;

    for theme in &all_themes {
        let should_enable = form.themes.contains(&theme.name) || theme.name == form.default_theme;
        if should_enable && theme.status == 0 {
            SystemItem::enable_theme(&pool, &theme.name).await?;
        } else if !should_enable && theme.status == 1 {
            SystemItem::disable_theme(&pool, &theme.name).await?;
        }
    }

    crate::models::set_default_theme(&pool, &form.default_theme).await?;

    Ok(Redirect::to("/admin/themes"))
}

// Statistics/Logs administration
pub async fn logs_hits(
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

    let stats_enabled = SystemItem::is_module_enabled(&pool, "statistics").await?;
    let hits = if stats_enabled {
        AccessLog::recent_hits(&pool, 50).await?
    } else {
        vec![]
    };

    let mut context = tera::Context::new();
    context.insert("title", "Recent hits");
    context.insert("current_user", &Some(user));
    context.insert("hits", &hits);
    context.insert("stats_enabled", &stats_enabled);

    let html = tera.render("admin/logs_hits.html", &context)?;
    Ok(Html(html))
}

pub async fn logs_pages(
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

    let stats_enabled = SystemItem::is_module_enabled(&pool, "statistics").await?;
    let pages = if stats_enabled {
        AccessLog::top_pages(&pool, 50).await?
    } else {
        vec![]
    };

    let mut context = tera::Context::new();
    context.insert("title", "Top pages");
    context.insert("current_user", &Some(user));
    context.insert("pages", &pages);
    context.insert("stats_enabled", &stats_enabled);

    let html = tera.render("admin/logs_pages.html", &context)?;
    Ok(Html(html))
}

pub async fn logs_visitors(
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

    let stats_enabled = SystemItem::is_module_enabled(&pool, "statistics").await?;
    let visitors = if stats_enabled {
        AccessLog::top_visitors(&pool, 50).await?
    } else {
        vec![]
    };

    let mut context = tera::Context::new();
    context.insert("title", "Top visitors");
    context.insert("current_user", &Some(user));
    context.insert("visitors", &visitors);
    context.insert("stats_enabled", &stats_enabled);

    let html = tera.render("admin/logs_visitors.html", &context)?;
    Ok(Html(html))
}

pub async fn logs_referrers(
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

    let stats_enabled = SystemItem::is_module_enabled(&pool, "statistics").await?;
    let referrers = if stats_enabled {
        AccessLog::top_referrers(&pool, 50).await?
    } else {
        vec![]
    };

    let mut context = tera::Context::new();
    context.insert("title", "Top referrers");
    context.insert("current_user", &Some(user));
    context.insert("referrers", &referrers);
    context.insert("stats_enabled", &stats_enabled);

    let html = tera.render("admin/logs_referrers.html", &context)?;
    Ok(Html(html))
}

pub async fn logs_access_detail(
    State(pool): State<MySqlPool>,
    State(tera): State<Tera>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Path(aid): Path<u32>,
) -> AppResult<Html<String>> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    if !user.has_permission(&pool, "administer nodes").await? {
        return Err(AppError::Forbidden);
    }

    let entry = AccessLog::find_by_aid(&pool, aid).await?;

    let mut context = tera::Context::new();
    context.insert("title", "Access log detail");
    context.insert("current_user", &Some(user));
    context.insert("entry", &entry);

    let html = tera.render("admin/logs_detail.html", &context)?;
    Ok(Html(html))
}

pub async fn statistics_settings_form(
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

    let enable_access_log = Variable::get_or_default(&pool, "statistics_enable_access_log", "0").await;
    let count_content_views = Variable::get_or_default(&pool, "statistics_count_content_views", "0").await;

    let mut context = tera::Context::new();
    context.insert("title", "Statistics settings");
    context.insert("current_user", &Some(user));
    context.insert("enable_access_log", &(enable_access_log == "1"));
    context.insert("count_content_views", &(count_content_views == "1"));

    let html = tera.render("admin/statistics_settings.html", &context)?;
    Ok(Html(html))
}

#[derive(Debug, Deserialize)]
pub struct StatisticsSettingsForm {
    #[serde(default)]
    pub enable_access_log: Option<String>,
    #[serde(default)]
    pub count_content_views: Option<String>,
}

pub async fn statistics_settings_submit(
    State(pool): State<MySqlPool>,
    Extension(CurrentUser(current_user)): Extension<CurrentUser>,
    Form(form): Form<StatisticsSettingsForm>,
) -> AppResult<Redirect> {
    let Some(user) = current_user else {
        return Err(AppError::Unauthorized);
    };

    if !user.has_permission(&pool, "administer nodes").await? {
        return Err(AppError::Forbidden);
    }

    let enable_access_log = if form.enable_access_log.is_some() { "1" } else { "0" };
    let count_content_views = if form.count_content_views.is_some() { "1" } else { "0" };

    Variable::set(&pool, "statistics_enable_access_log", enable_access_log).await?;
    Variable::set(&pool, "statistics_count_content_views", count_content_views).await?;

    Ok(Redirect::to("/admin/logs/settings"))
}
