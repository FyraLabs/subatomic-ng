pub mod rpm;
pub mod tag;
pub mod gpg_key;
use std::sync::LazyLock;

use surrealdb::{
    engine::remote::ws::{Client, Ws},
    opt::auth::Root,
    Surreal,
};

pub static DB: SurrealClient = SurrealClient::new();

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

    let schemas = vec![
        include_str!("schema/rpm.surql"),
        include_str!("schema/tag.surql"),
        include_str!("schema/available_pkgs.surql"),
        include_str!("schema/event_log.surql"),
    ];

    DB.use_ns(namespace).use_db(db).await?;

    // todo: schema migration
    for schema in schemas {
        DB.query(schema).await?;
    }

    // println!("{:?}", q);
    Ok(())
}
