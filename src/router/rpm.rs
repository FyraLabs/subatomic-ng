use crate::errors::Result;
use crate::obj_store::object_store;
use axum::debug_handler;
use axum::extract::{Json, Query};
use axum::{
    extract::{Multipart, Path},
    http::StatusCode,
    routing::{delete, get, post, put},
    Router,
};
use serde::Deserialize;
use ulid::Ulid;

use crate::config::CONFIG;
use crate::db::rpm::{Rpm, RpmRef};

pub fn route() -> Router {
    Router::new()
        .route("/rpms", get(get_all_rpms))
        .nest("/rpm", route_operations())
}

fn route_operations() -> Router {
    Router::new()
        .route("/{ulid}", get(get_rpm))
        .route("/{ulid}", delete(delete_rpm))
        .route("/{ulid}/available", post(mark_rpm_available))
        .route("/{ulid}/available", delete(mark_rpm_unavailable))
        .route("/upload", put(upload_rpm))
}
#[derive(Debug, Deserialize)]
pub struct RpmUploadParams {
    prune: bool,
}
pub async fn get_rpm(Path(pkg_id): Path<Ulid>) -> Result<Json<Rpm>> {
    let rpm = Rpm::get(pkg_id).await?.unwrap();
    Ok(Json(rpm))
}


pub async fn get_all_rpms() -> Result<Json<Vec<RpmRef>>> {
    let rpms = Rpm::get_all().await?;
    Ok(Json(rpms.into_iter().map(|r| RpmRef::from(&r)).collect()))
}

pub async fn mark_rpm_available(Path(pkg_id): Path<Ulid>) -> Result<StatusCode> {
    let rpm = Rpm::get(pkg_id).await?.unwrap();
    rpm.mark_available().await?;
    Ok(StatusCode::OK)
}

pub async fn mark_rpm_unavailable(Path(pkg_id): Path<Ulid>) -> Result<StatusCode> {
    let rpm = Rpm::get(pkg_id).await?.unwrap();
    rpm.mark_unavailable().await?;
    Ok(StatusCode::OK)
}

pub async fn delete_rpm(Path(pkg_id): Path<Ulid>) -> Result<StatusCode> {
    let rpm = Rpm::get(pkg_id).await?.unwrap();
    rpm.delete().await?;
    Ok(StatusCode::OK)
}
#[debug_handler]
pub async fn upload_rpm(
    Query(params): Query<RpmUploadParams>,
    mut multipart: Multipart,
) -> Result<StatusCode> {
    let mut filename = None;
    let mut data = None;

    let mut tag = None;

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name();
        if name == Some("file_upload") {
            filename = field.file_name().map(|f| f.to_string());
            data = field.bytes().await.ok();
        } else if name == Some("id") || name == Some("tag") {
            tag = field.text().await.ok();
        }
    }

    if let (Some(filename), Some(data), Some(tag)) = (filename, data, tag) {
        let objstore = object_store();
        tracing::info!("filename: {:?}", filename);
        // tracing::info!("data: {:?}", data);
        let dest = CONFIG.get().unwrap().cache_dir.join(filename);
        tracing::info!("dest: {:?}", dest);

        tokio::fs::write(&dest, &data).await?;

        let rpm = Rpm::from_path(&dest, &tag)?;
        tracing::trace!("RPM: {:?}", rpm);

        // Now push and upload to object store & cache

        objstore.put(&rpm.object_key, &dest).await?;

        // Now commit to db

        rpm.commit_to_db(params.prune).await?;

        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::from_u16(400).unwrap())
    }

    // StatusCode::from_u16(500).unwrap()
}
