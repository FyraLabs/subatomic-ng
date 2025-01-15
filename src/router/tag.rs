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

pub fn route(router: Router) -> Router {
    router
        .route("/repo", post(create_tag))
        .route("/repos", get(get_all_tags))
        .route("/repo/{id}", get(get_tag))
        .route("/repo/{id}", delete(delete_tag))
        .route("/repo/{id}/rpms", get(get_tag_rpms))
        .route("/repo", get(get_all_tags))
        .route("/repo/{id}/assemble", post(assemble_tag))
    // .route("/tag/{id}/comps", put(upload_tag))
}

pub async fn get_tag(Path(tag_id): Path<String>) -> Json<Tag> {
    let tag = Tag::get(&tag_id).await.unwrap().unwrap();
    Json(tag)
}

pub async fn get_tag_rpms(Path(tag_id): Path<String>) -> Json<Vec<RpmRef>> {
    let tag = Tag::get(&tag_id).await.unwrap().unwrap();
    let rpms = tag.get_available_rpms().await.unwrap();
    let rpms = rpms.iter().map(|r| r.into()).collect();
    Json(rpms)
}

pub async fn get_all_tags() -> Json<Vec<Tag>> {
    let tags = Tag::get_all().await.unwrap();
    Json(tags)
}

pub async fn create_tag(tag: Json<CreateTag>) -> Json<Tag> {
    let tag = Tag::new(tag.name.clone());
    let tag = tag.save().await.unwrap();
    Json(tag)
}

pub async fn delete_tag(Path(tag_id): Path<String>) -> StatusCode {
    let tag = Tag::get(&tag_id).await.unwrap().unwrap();
    let r = tag.delete().await;

    if r.is_ok() {
        return StatusCode::from_u16(200).unwrap();
    }

    StatusCode::from_u16(500).unwrap()
}

pub async fn assemble_tag(Path(tag_id): Path<String>) -> StatusCode {
    let tag = Tag::get(&tag_id).await.unwrap().unwrap();
    let r = tag.assemble().await;

    match r {
        Ok(_) => StatusCode::from_u16(200).unwrap(),
        Err(e) => {
            tracing::error!("error assembling tag: {:#?}", e);
            StatusCode::from_u16(500).unwrap()
        }
    }
}
