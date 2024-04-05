use crate::prelude::Config;
use chrono::{Duration, Utc};
use integrationos_domain::{Claims, IntegrationOSError as Error, InternalError};
use jsonwebtoken::{encode, EncodingKey, Header};

pub trait TokenGenerator {
    fn generate(&self, configuration: Config, expiration: i64) -> Result<String, Error>;
}

#[derive(Debug, Default)]
pub struct JwtTokenGenerator;

impl TokenGenerator for JwtTokenGenerator {
    fn generate(&self, configuration: Config, expiration: i64) -> Result<String, Error> {
        let key = configuration.server().admin_secret();
        let key = key.as_bytes();
        let key = EncodingKey::from_secret(key);
        let now = Utc::now();
        let iat = now.timestamp();
        let exp = (now + Duration::days(expiration)).timestamp();
        let header = Header::default();

        let claims = Claims {
            id: "ADMIN".into(),
            email: "admin@integrationos.com".into(),
            username: "admin".into(),
            user_key: "admin".into(),
            first_name: "admin".into(),
            last_name: "admin".into(),
            buildable_id: "".into(),
            container_id: "".into(),
            pointers: vec![],
            is_buildable_core: false,
            iat,
            exp,
            aud: "integration-team".into(),
            iss: "oauth-integrationos".into(),
        };

        let token = encode(&header, &claims, &key)
            .map_err(|e| InternalError::encryption_error(e.to_string().as_str(), None))?;

        Ok(token)
    }
}
