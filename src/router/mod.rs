use crate::cache::Cache;
use crate::config::CONFIG;
use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::Json;
use axum::Router;
pub mod rpm;
pub mod tag;

pub fn route(router: Router) -> Router {
    let router = rpm::route(router);
    tag::route(router)
}
