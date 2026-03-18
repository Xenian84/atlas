use thiserror::Error;

#[derive(Debug, Error)]
pub enum AtlasError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("unauthorized")]
    Unauthorized,

    #[error("rate limited")]
    RateLimited,

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl AtlasError {
    pub fn http_status(&self) -> u16 {
        match self {
            AtlasError::NotFound(_)   => 404,
            AtlasError::Unauthorized  => 401,
            AtlasError::RateLimited   => 429,
            AtlasError::BadRequest(_) => 400,
            _                         => 500,
        }
    }
}
