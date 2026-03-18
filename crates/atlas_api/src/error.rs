use axum::{response::{IntoResponse, Response}, http::StatusCode, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("rate limited")]
    RateLimited,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("serialization error: {0}")]
    SerdeJson(#[from] serde_json::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            ApiError::NotFound(m)   => (StatusCode::NOT_FOUND,              m.clone()),
            ApiError::Unauthorized  => (StatusCode::UNAUTHORIZED,           "unauthorized".into()),
            ApiError::RateLimited   => (StatusCode::TOO_MANY_REQUESTS,      "rate limited".into()),
            ApiError::BadRequest(m) => (StatusCode::BAD_REQUEST,            m.clone()),
            ApiError::Internal(e)   => (StatusCode::INTERNAL_SERVER_ERROR,  e.to_string()),
            ApiError::Db(e)         => (StatusCode::INTERNAL_SERVER_ERROR,  e.to_string()),
            ApiError::SerdeJson(e)  => (StatusCode::INTERNAL_SERVER_ERROR,  e.to_string()),
        };
        (status, Json(json!({ "error": msg }))).into_response()
    }
}
