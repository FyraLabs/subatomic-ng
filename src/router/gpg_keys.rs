use axum::{
    extract::{Multipart, Path},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};