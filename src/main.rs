use axum::{
    extract::Path,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use clap::Parser;
use db::rpm::Rpm;
use obj_store::OBJECT_STORE;
use serde::{Deserialize, Serialize};
use ulid::Ulid;
mod cache;
mod config;
mod db;
mod errors;
mod obj_store;
mod router;

#[tokio::main]
async fn main() {
    // initialize tracing
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    let cfg = config::Config::init();
    // config::Config::parse();

    db::connect_db(&cfg.surreal_ns, &cfg.surreal_db)
        .await
        .unwrap();

    println!("{:#?}", cache::cache().list_cached().await.unwrap());

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root));
    let app = router::route(app);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind(cfg.listen_addr)
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}
// path payload
