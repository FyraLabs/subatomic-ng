//! Tag (Repo management) routes for Subatomic-NG
//!
//!
//! Tags are similar to the old subatomic's repos, but with a few differences:
//!
//! - Tags are now versioned, and may have multiple versions
//! - Artifacts are now stored in object storage, not the exported directory, but are still cached locally for serving
//! - Unavailable artifacts are no longer deleted, but marked as such
//! - Exported repos are now rebuilt from scratch when a new artifact is marked available
use axum::{
    extract::Path,
    http::StatusCode,
    response::Json,
    routing::{delete, get, post},
    Router,
};

use axum_error_handler::AxumErrorResponse;
use thiserror::Error;
#[derive(Error, Debug, AxumErrorResponse)]
pub enum TagError {
    #[error("Tag not found")]
    #[status_code("NOT_FOUND")]
    NotFound,
    #[error("Tag already exists")]
    #[status_code("CONFLICT")]
    AlreadyExists,
}

use crate::errors::Result;

// single enum for now
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoType {
    Rpm,
}

use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTag {
    name: String,
    #[serde(rename = "type")]
    repo_type: RepoType,
}

use crate::db::{rpm::RpmRef, tag::Tag};

pub fn route() -> Router {
    Router::new()
        .route("/repos", get(get_all_tags))
        .nest("/repo", route_operations())
}

fn route_operations() -> Router {
    Router::new()
        .route("/", post(create_tag))
        .route("/", get(get_all_tags))
        .route("/{id}", get(get_tag))
        .route("/{id}", delete(delete_tag))
        .route("/{id}/key", post(set_gpg_key))
        .route("/{id}/rpms", get(get_tag_rpms))
        .route("/{id}/assemble", post(assemble_tag))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetGpgKey {
    key_id: String,
}

pub async fn get_tag(Path(tag_id): Path<String>) -> Result<Json<Tag>> {
    let tag = Tag::get(&tag_id)
        .await?
        .ok_or_else(|| TagError::NotFound)?;
    Ok(Json(tag))
}

pub async fn set_gpg_key(
    Path(tag_id): Path<String>,
    Json(key): Json<SetGpgKey>,
) -> Result<Json<Tag>> {
    let mut tag = Tag::get(&tag_id)
        .await?
        .ok_or_else(|| TagError::NotFound)?;
    let key = key.key_id;
    tag.set_gpg_key(&key);

    Ok(Json(tag.save().await?))
}

pub async fn get_tag_rpms(Path(tag_id): Path<String>) -> Result<Json<Vec<RpmRef>>> {
    let tag = Tag::get(&tag_id)
        .await?
        .ok_or_else(|| crate::errors::Error::NotFound)?;
    let rpms = tag.get_available_rpms().await?;
    let rpms = rpms.iter().map(|r| r.into()).collect();
    Ok(Json(rpms))
}

pub async fn get_all_tags() -> Result<Json<Vec<Tag>>> {
    let tags = Tag::get_all().await?;
    Ok(Json(tags))
}
// #[debug_handler]
pub async fn create_tag(tag: Json<CreateTag>) -> Result<(StatusCode, Json<Tag>)> {
    let existing_tag = Tag::get(&tag.name).await?;
    if existing_tag.is_some() {
        return Err(TagError::AlreadyExists.into());
    }
    let tag = Tag::new(tag.name.clone());

    Ok((StatusCode::CREATED, Json(tag.save().await?)))
}

pub async fn delete_tag(Path(tag_id): Path<String>) -> Result<StatusCode> {
    let tag = Tag::get(&tag_id)
        .await?
        .ok_or_else(|| crate::errors::Error::NotFound)?;
    tag.delete().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn assemble_tag(Path(tag_id): Path<String>) -> Result<StatusCode> {
    let tag = Tag::get(&tag_id)
        .await?
        .ok_or_else(|| crate::errors::Error::NotFound)?;
    tag.assemble().await?;
    Ok(StatusCode::ACCEPTED)
}
