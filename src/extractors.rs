use axum::{
    async_trait,
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use serde::de::DeserializeOwned;

pub struct QsForm<T>(pub T);

#[async_trait]
impl<S, T> FromRequest<S> for QsForm<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = QsFormRejection;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|_| QsFormRejection::BytesRejection)?;

        let value = serde_qs::from_bytes(&bytes)
            .map_err(|e| QsFormRejection::FailedToDeserialize(e.to_string()))?;

        Ok(QsForm(value))
    }
}

pub enum QsFormRejection {
    BytesRejection,
    FailedToDeserialize(String),
}

impl IntoResponse for QsFormRejection {
    fn into_response(self) -> Response {
        match self {
            QsFormRejection::BytesRejection => {
                (StatusCode::BAD_REQUEST, "Failed to read request body").into_response()
            }
            QsFormRejection::FailedToDeserialize(e) => {
                (StatusCode::UNPROCESSABLE_ENTITY, format!("Failed to deserialize form body: {}", e)).into_response()
            }
        }
    }
}
