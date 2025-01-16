use axum::{http::Response, routing::get, Router};
use color_eyre::eyre::eyre;
use db::DB;
use errors::Error;
use pgp::VERSION;
mod cache;
mod config;
mod db;
mod errors;
mod obj_store;
mod router;
use std::{net::SocketAddr, str::FromStr};



fn router() -> Router {
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/version", get(version));
    router::route(app)
}

#[tokio::main]
async fn main() {
    // initialize tracing
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();
    let cfg = config::Config::init();

    db::connect_db(&cfg.surreal_ns, &cfg.surreal_db)
        .await
        .unwrap();

    let app = router();
    // run our app with hyper, listening globally on port 3000
    let addr = SocketAddr::from_str(&cfg.listen_addr).unwrap();
    
    axum_server::bind(addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

/// Returns the version of the server
async fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Returns the health of the server
async fn health() -> Result<&'static str, Error> {
    let h = DB.get().health().await.is_ok();
    
    if h {
        Ok("OK")
    } else {
        Err(Error::Other(eyre!("health check failed")))
    }
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}
// path payload
