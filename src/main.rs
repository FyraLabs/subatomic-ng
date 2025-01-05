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
    const FILE_PATH: &str = "test/data/anda-srpm-macros-0:0.2.6-1.fc41.noarch.rpm";

    // let rpm = Rpm::from_path(FILE_PATH).unwrap();
    // rpm.commit_to_db().await.unwrap();

    // OBJECT_STORE
    //     .get()
    //     .unwrap()
    //     .put_file(&rpm.object_key, FILE_PATH.into())
    //     .await
    //     .unwrap();

    // build our application with a route
    let app = Router::new()
        // `GET /` goes to `root`
        .route("/", get(root))
        // `POST /users` goes to `create_user`
        .route("/users", post(create_user));
    let app = router::route(app);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// basic handler that responds with a static string
async fn root() -> &'static str {
    "Hello, World!"
}
// path payload
async fn get_rpm(Path(rpm): Path<Ulid>) -> String {
    format!("RPM: {}", rpm)
}

async fn create_user(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
    // insert your application logic here
    let user = User {
        id: 1337,
        username: payload.username,
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}

// the input to our `create_user` handler
#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

// the output to our `create_user` handler
#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}
