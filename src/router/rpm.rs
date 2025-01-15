use axum::{
    extract::{Multipart, Path},
    http::StatusCode,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use ulid::Ulid;
use crate::obj_store::object_store;

use crate::config::CONFIG;
use crate::db::rpm::Rpm;

pub fn route(router: Router) -> Router {
    router
        .route("/rpm/{ulid}", get(get_rpm))
        .route("/rpm/{ulid}", delete(delete_rpm))
        .route("/rpm/{ulid}/available", post(mark_rpm_available))
        .route("/rpm/{ulid}/available", delete(mark_rpm_unavailable))
        .route("/rpms", get(get_all_rpms))
        .route("/rpm/upload", put(upload_rpm))
}

pub async fn get_rpm(Path(pkg_id): Path<Ulid>) -> Json<Rpm> {
    let rpm = Rpm::get(pkg_id).await.unwrap().unwrap();
    Json(rpm)
}

pub async fn get_all_rpms() -> Json<Vec<Rpm>> {
    let rpms = Rpm::get_all().await.unwrap();
    Json(rpms)
}

pub async fn mark_rpm_available(Path(pkg_id): Path<Ulid>) -> StatusCode {
    let rpm = Rpm::get(pkg_id).await.unwrap().unwrap();
    let r = rpm.mark_available().await;

    if r.is_ok() {
        return StatusCode::from_u16(200).unwrap();
    }

    StatusCode::from_u16(500).unwrap()
}

pub async fn mark_rpm_unavailable(Path(pkg_id): Path<Ulid>) -> StatusCode {
    let rpm = Rpm::get(pkg_id).await.unwrap().unwrap();
    let r = rpm.mark_unavailable().await;

    if r.is_ok() {
        return StatusCode::from_u16(200).unwrap();
    }

    StatusCode::from_u16(500).unwrap()
}

pub async fn delete_rpm(Path(pkg_id): Path<Ulid>) -> StatusCode {
    let rpm = Rpm::get(pkg_id).await.unwrap().unwrap();
    let r = rpm.delete().await;

    if r.is_ok() {
        return StatusCode::from_u16(200).unwrap();
    }

    StatusCode::from_u16(500).unwrap()
}

pub async fn upload_rpm(mut multipart: Multipart) -> StatusCode {
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

        if let Err(e) = tokio::fs::write(&dest, data).await {
            tracing::error!("failed to write file: {:?}", e);
            return StatusCode::from_u16(500).unwrap();
        }

        let rpm = match Rpm::from_path(&dest, &tag) {
            Ok(rpm) => rpm,
            Err(_) => return StatusCode::from_u16(500).unwrap(),
        };
        tracing::trace!("RPM: {:?}", rpm);

        // Now push and upload to object store & cache

        objstore.put(&rpm.object_key, &dest).await.unwrap();

        // Now commit to db

        let r = rpm.commit_to_db(true).await;

        if let Ok(r) = r {
            return StatusCode::from_u16(200).unwrap();
        } else {
            tracing::error!("failed to commit to db: {:?}", r);
            return StatusCode::from_u16(500).unwrap();
        }

    } else {
        StatusCode::from_u16(400).unwrap()
    }

    // StatusCode::from_u16(500).unwrap()
}
