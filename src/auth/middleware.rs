use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use sqlx::MySqlPool;
use tower_sessions::Session;

use crate::models::{session::SESSION_USER_KEY, User};

#[derive(Clone)]
pub struct CurrentUser(pub Option<User>);

pub async fn auth_middleware(
    State(pool): State<MySqlPool>,
    session: Session,
    mut request: Request,
    next: Next,
) -> Response {
    let user = match session.get::<u32>(SESSION_USER_KEY).await {
        Ok(Some(uid)) => User::find_by_uid(&pool, uid).await.ok().flatten(),
        _ => None,
    };

    request.extensions_mut().insert(CurrentUser(user));
    next.run(request).await
}
