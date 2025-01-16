//! API routes for Subatomic's Keyring
//! The keyring is used to sign and verify RPMs managed by Subatomic.


use axum::{
    extract::{Multipart, Path},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};

use crate::{config::CONFIG, db::gpg_key};
use crate::errors::Result;
use crate::db::gpg_key::GpgKeyRef;
use serde::{Deserialize, Serialize};

pub fn route() -> Router {
    Router::new()
        .route("/keys", get(get_all_keys))
        .nest("/key", route_operations())
}

fn route_operations() -> Router {
    Router::new()
        .route("/", post(create_key))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGpgKey {
    /// The ID of the key in the keyring
    pub id: String,
    /// The user ID of the key, i.e `John Doe <john@example.com>`
    pub user_id: String,
    /// Optional description of the key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}


pub async fn get_all_keys() -> Result<Json<Vec<GpgKeyRef>>> {
    let keys = gpg_key::GpgKey::get_all().await?;
    Ok(Json(keys.into_iter().map(|r| GpgKeyRef::from(&r)).collect()))
}

pub async fn create_key(Json(key): Json<CreateGpgKey>) -> Result<Json<GpgKeyRef>> {
    let key = gpg_key::GpgKey::new(&key.id, key.description, &key.user_id)?;
    
    Ok(Json(GpgKeyRef::from(&key.save().await?)))
}