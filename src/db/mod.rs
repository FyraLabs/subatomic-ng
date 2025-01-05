pub mod tag;
pub mod rpm;

use std::sync::LazyLock;

use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
    Surreal,
};

static DB: SurrealClient = SurrealClient::new();

pub struct SurrealClient {
    pub db: LazyLock<Surreal<Client>>,
}

impl std::ops::Deref for SurrealClient {
    type Target = Surreal<Client>;
    fn deref(&self) -> &Self::Target {
        &self.db
    }
}

impl SurrealClient {
    const fn new() -> Self {
        SurrealClient {
            db: LazyLock::new(Surreal::init),
        }
    }

    pub fn get(&self) -> &Surreal<Client> {
        &DB
    }

    pub async fn connect_ws(&self, addr: &str) -> color_eyre::Result<()> {
        self.get().connect::<Ws>(addr).await?;
        Ok(())
    }
}

// TODO: should use Surreal<Any>
pub async fn connect_db(namespace: &str, db: &str) -> color_eyre::Result<()> {
    DB.connect::<Ws>("localhost:8000").await?;

    DB.signin(Root {
        username: "root",
        password: "root",
    })
    .await?;

    DB.use_ns(namespace).use_db(db).await?;

    let q = DB
        .query(
            "
    DEFINE TABLE IF NOT EXISTS person SCHEMALESS
        PERMISSIONS FOR 
            CREATE, SELECT WHERE $auth,
            FOR UPDATE, DELETE WHERE created_by = $auth;
    DEFINE FIELD IF NOT EXISTS name ON TABLE person TYPE string;
    DEFINE FIELD IF NOT EXISTS created_by ON TABLE person VALUE $auth READONLY;

    DEFINE INDEX IF NOT EXISTS unique_name ON TABLE user FIELDS name UNIQUE;
    DEFINE ACCESS IF NOT EXISTS account ON DATABASE TYPE RECORD
    SIGNUP ( CREATE user SET name = $name, pass = crypto::argon2::generate($pass) )
    SIGNIN ( SELECT * FROM user WHERE name = $name AND crypto::argon2::compare(pass, $pass) )
    DURATION FOR TOKEN 15m, FOR SESSION 12h
    ;",
        )
        .await?;

    println!("{:?}", q);
    Ok(())
}
