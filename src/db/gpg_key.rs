use color_eyre::{eyre::ContextCompat, Result};
use pgp::{types::SecretKeyTrait, ArmorOptions, Deserializable, SecretKeyParamsBuilder};
use serde::{Deserialize, Serialize};
use surrealdb::sql::{Datetime, Thing};

use super::DB;
pub const GPG_KEY_TABLE: &str = "gpg_key";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpgKeyRef {
    pub id: String,
    pub user_id: String,
    pub description: Option<String>,
    pub public_key: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// When querying, we should return a GPGKeyRef instead for security reasons
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpgKey {
    pub id: Thing,
    pub description: Option<String>,
    pub user_id: String,
    /// Armored secret key
    pub secret_key: String,
    /// Armored public key
    pub public_key: String,
    pub created_at: surrealdb::sql::Datetime,
}

impl From<&GpgKey> for GpgKeyRef {
    fn from(key: &GpgKey) -> Self {
        GpgKeyRef {
            id: key.id.id.to_string(),
            user_id: key.user_id.clone(),
            description: key.description.clone(),
            public_key: key.public_key.clone(),
            created_at: key.created_at.to_utc(),
        }
    }
}

impl GpgKey {
    #[tracing::instrument]
    pub fn new(id: &str, description: Option<String>, user_id: &str) -> Result<Self> {
        let secret_key = SecretKeyParamsBuilder::default()
            .can_certify(false)
            .key_type(pgp::KeyType::Ed25519)
            .can_sign(true)
            .primary_user_id(user_id.to_owned())
            .build()?;

        let mut thread_rng = rand::thread_rng();
        let secret_key = secret_key.generate(&mut thread_rng)?;
        let passwd_fn = || String::new();
        let signed_secret_key = secret_key.sign(&mut thread_rng, passwd_fn)?;

        let secret_key_armored = signed_secret_key.to_armored_string(ArmorOptions::default())?;
        let public_key_armored = signed_secret_key
            .public_key()
            .sign(&mut rand::thread_rng(), &signed_secret_key, passwd_fn)?
            .to_armored_string(ArmorOptions::default())?;

        Ok(GpgKey {
            id: Thing::from((GPG_KEY_TABLE, id)),
            description,
            user_id: user_id.to_owned(),
            secret_key: secret_key_armored,
            public_key: public_key_armored,
            created_at: Datetime::default(),
        })
    }

    #[tracing::instrument]
    pub fn secret_key(&self) -> Result<pgp::SignedSecretKey> {
        let (key, _headers) = pgp::SignedSecretKey::from_string(&self.secret_key)?;
        Ok(key)
    }

    #[tracing::instrument]
    pub fn public_key(&self) -> Result<pgp::SignedPublicKey> {
        let (key, _headers) = pgp::SignedPublicKey::from_string(&self.public_key)?;
        Ok(key)
    }
    
    #[tracing::instrument]
    pub async fn save(&self) -> Result<Self> {
        let query = DB
            .upsert((GPG_KEY_TABLE, self.id.id.to_raw()))
            .content(self.clone())
            .await?;
        

        query.context("nothing returned from insert")
    }
    
    #[tracing::instrument]
    pub async fn get(id: &str) -> Result<Option<Self>> {
        Ok(DB.select((GPG_KEY_TABLE, id)).await?)
    }
    
    #[tracing::instrument]
    pub async fn delete(&self) -> Result<()> {
        DB
            .delete((GPG_KEY_TABLE, self.id.id.to_raw()))
            .await?
            .map_or(Ok(()), Ok)
    }
    
    #[tracing::instrument]
    pub async fn get_all() -> Result<Vec<Self>> {
        Ok(DB.select(GPG_KEY_TABLE).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // use spectral::prelude::*;
    #[test]
    fn test_new_gpg_key() {
        let key = GpgKey::new("test", None, "test").unwrap();
        println!("{:?}", key);

        let key_ref = GpgKeyRef::from(&key);

        println!("{:?}", key_ref);
    }
}
