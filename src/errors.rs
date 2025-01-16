use axum::http::StatusCode;
use thiserror::Error;
use axum_thiserror::ErrorStatus;
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug, ErrorStatus)]
pub enum Error {
    #[error("database error")]
    #[status(StatusCode::INTERNAL_SERVER_ERROR)]
    Db(#[from] surrealdb::Error),
    
    // other error
    #[error("error: {0}")]
    #[status(StatusCode::INTERNAL_SERVER_ERROR)]
    Other(#[from] color_eyre::Report),
    
    #[error("Server I/O error")]
    #[status(StatusCode::INTERNAL_SERVER_ERROR)]
    Io(#[from] std::io::Error),
    
    #[error("Not Found")]
    #[status(StatusCode::NOT_FOUND)]
    NotFound,
}
