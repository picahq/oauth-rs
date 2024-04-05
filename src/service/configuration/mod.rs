mod telemetry;

use actix_governor::{KeyExtractor, PeerIpKeyExtractor, SimpleKeyExtractionError};
use actix_web::dev::ServiceRequest;
pub use telemetry::*;

use envconfig::Envconfig;
use integrationos_domain::{
    database::DatabaseConfig, environment::Environment, secrets::SecretsConfig,
};
use std::collections::HashMap;
use std::fmt::Debug;
use std::net::IpAddr;

#[derive(Clone, Envconfig)]
pub struct OAuthConfig {
    #[envconfig(env = "REFRESH_BEFORE_IN_MINUTES", default = "10")]
    refresh_before: i64,
    #[envconfig(env = "SLEEP_TIMER_IN_SECONDS", default = "20")]
    sleep_timer: u64,
    #[envconfig(nested = true)]
    database: DatabaseConfig,
    #[envconfig(nested = true)]
    secrets_config: SecretsConfig,
}

impl Debug for OAuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuthConfig")
            .field("refresh_before", &self.refresh_before)
            .field("sleep_timer", &self.sleep_timer)
            .finish()
    }
}

impl OAuthConfig {
    pub fn refresh_before(&self) -> i64 {
        self.refresh_before
    }

    pub fn sleep_timer(&self) -> u64 {
        self.sleep_timer
    }

    pub fn database(&self) -> &DatabaseConfig {
        &self.database
    }

    pub fn secrets_config(&self) -> &SecretsConfig {
        &self.secrets_config
    }

    pub fn load() -> Result<Self, envconfig::Error> {
        // dotenv().ok() is already called in the main.rs
        OAuthConfig::init_from_env()
    }
}

#[derive(Clone, Envconfig, Debug)]
pub struct ServerConfig {
    #[envconfig(from = "ENVIRONMENT", default = "test")]
    /// The environment for the server
    environment: Environment,
    #[envconfig(from = "HOST", default = "localhost")]
    /// The host for the server
    host: String,
    #[envconfig(from = "PORT", default = "3007")]
    /// The port for the server
    port: u16,
    #[envconfig(from = "APP_URL", default = "http://localhost:3007")]
    /// The URL for the server
    app_url: String,
    #[envconfig(
        from = "JWT_SECRET",
        default = "2thZ2UiOnsibmFtZSI6IlN0YXJ0dXBsa3NoamRma3NqZGhma3NqZGhma3NqZG5jhYtggfaP9ubmVjdGlvbnMiOjUwMDAwMCwibW9kdWxlcyI6NSwiZW5kcG9pbnRzIjo3b4e05e2-f050-401f-9822-44f43f71753c"
    )]
    /// The secret for the JWT
    jwt_secret: String,
    #[envconfig(from = "TIMEOUT", default = "30000")]
    timeout: u64,
    #[envconfig(
        from = "SECRET_ADMIN",
        default = "my_admin_secret_super_extra_secure_key_to_verify_admin_sessions_this_one_must_be_at_least_51_characters"
    )]
    admin_secret: String,
    /// Burst rate limit
    #[envconfig(from = "BURST_RATE_LIMIT", default = "10")]
    burst_rate_limit: u64,
    /// Burst size limit
    #[envconfig(from = "BURST_SIZE_LIMIT", default = "15")]
    burst_size_limit: u32,
    #[envconfig(from = "HEADER_AUTH", default = "x-integrationos-secret")]
    pub auth_header: String,
    #[envconfig(from = "HEADER_ADMIN", default = "x-integrationos-admin-token")]
    pub admin_header: String,
    #[envconfig(from = "CACHE_SIZE", default = "10000")]
    pub cache_size: u64,
}

impl ServerConfig {
    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn environment(&self) -> &Environment {
        &self.environment
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn app_url(&self) -> &str {
        &self.app_url
    }

    pub fn is_development(&self) -> bool {
        self.environment == Environment::Development || self.environment == Environment::Test
    }

    pub fn jwt_secret(&self) -> &str {
        &self.jwt_secret
    }

    pub fn timeout(&self) -> u64 {
        self.timeout
    }

    pub fn cache_size(&self) -> u64 {
        self.cache_size
    }

    pub fn burst_rate_limit(&self) -> u64 {
        self.burst_rate_limit
    }

    pub fn auth_header(&self) -> &str {
        &self.auth_header
    }

    pub fn burst_size_limit(&self) -> u32 {
        self.burst_size_limit
    }

    pub fn admin_header(&self) -> &str {
        &self.admin_header
    }

    pub fn admin_secret(&self) -> &str {
        &self.admin_secret
    }

    pub fn load() -> Result<Self, envconfig::Error> {
        // dotenv().ok() is already called in the main.rs
        ServerConfig::init_from_env()
    }
}

#[derive(Clone)]
pub struct Config {
    oauth: OAuthConfig,
    server: ServerConfig,
}

impl Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let _ = f
            .debug_struct("OAuthConfig")
            .field("refresh_before", &self.oauth.refresh_before)
            .field("sleep_timer", &self.oauth.sleep_timer)
            .finish();

        writeln!(f)?;

        f.debug_struct("ServerConfig")
            .field("environment", &self.server.environment)
            .field("host", &self.server.host)
            .field("port", &self.server.port)
            .field("jwt_secret", &"[REDACTED]")
            .field("admin_secret", &"[REDACTED]")
            .field("app_url", &self.server.app_url)
            .field("timeout", &self.server.timeout)
            .field("burst_rate_limit", &self.server.burst_rate_limit)
            .field("burst_size_limit", &self.server.burst_size_limit)
            .field("auth_header", &self.server.auth_header)
            .field("cache_size", &self.server.cache_size)
            .finish()
    }
}

impl Config {
    pub fn new(oauth: OAuthConfig, server: ServerConfig) -> Self {
        Self { oauth, server }
    }

    pub fn oauth(&self) -> &OAuthConfig {
        &self.oauth
    }

    pub fn server(&self) -> &ServerConfig {
        &self.server
    }
}

impl From<HashMap<&str, &str>> for OAuthConfig {
    fn from(value: HashMap<&str, &str>) -> Self {
        let refresh_before = value
            .get("REFRESH_BEFORE_IN_MINUTES")
            .and_then(|value| value.parse().ok())
            .unwrap_or(10);

        let sleep_timer = value
            .get("SLEEP_TIMER_IN_SECONDS")
            .and_then(|value| value.parse().ok())
            .unwrap_or(20);

        let owned = value
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let database = DatabaseConfig::init_from_hashmap(&owned).unwrap_or_default();
        let secrets_config = SecretsConfig::init_from_hashmap(&owned).unwrap_or_default();

        Self {
            refresh_before,
            sleep_timer,
            database,
            secrets_config,
        }
    }
}

impl From<HashMap<&str, &str>> for ServerConfig {
    fn from(value: HashMap<&str, &str>) -> Self {
        let environment = value
            .get("ENVIRONMENT")
            .and_then(|value| value.parse().ok())
            .unwrap_or("test".parse().unwrap());
        let host = value.get("HOST").unwrap_or(&"localhost").to_string();
        let port = value
            .get("PORT")
            .and_then(|value| value.parse().ok())
            .unwrap_or(3008);
        let app_url = value
            .get("APP_URL")
            .unwrap_or(&"http://localhost:3008")
            .to_string();
        let jwt_secret = value
                .get("JWT_SECRET")
                .unwrap_or(
                    &"2thZ2UiOnsibmFtZSI6IlN0YXJ0dXBsa3NoamRma3NqZGhma3NqZGhma3NqZG5jhYtggfaP9ubmVjdGlvbnMiOjUwMDAwMCwibW9kdWxlcyI6NSwiZW5kcG9pbnRzIjo3b4e05e2-f050-401f-9822-44f43f71753c"
                )
                .to_string();
        let timeout = value
            .get("TIMEOUT")
            .and_then(|value| value.parse().ok())
            .unwrap_or(30000);
        let burst_rate_limit = value
            .get("BURST_RATE_LIMIT")
            .and_then(|value| value.parse().ok())
            .unwrap_or(10);
        let burst_size_limit = value
            .get("BURST_SIZE_LIMIT")
            .and_then(|value| value.parse().ok())
            .unwrap_or(15);
        let auth_header = value
            .get("HEADER_AUTH")
            .unwrap_or(&"x-integrationos-secret")
            .to_string();
        let cache_size = value
            .get("CACHE_SIZE")
            .and_then(|value| value.parse().ok())
            .unwrap_or(10000);
        let secret_admin = value
            .get("SECRET_ADMIN")
            .unwrap_or(
                &"my_admin_secret_super_extra_secure_key_to_verify_admin_sessions_this_one_must_be_at_least_51_characters"
            )
            .to_string();
        let admin_header = value
            .get("HEADER_ADMIN")
            .unwrap_or(&"x-integrationos-admin-token")
            .to_string();

        Self {
            environment,
            host,
            port,
            admin_header,
            app_url,
            jwt_secret,
            timeout,
            burst_rate_limit,
            admin_secret: secret_admin,
            burst_size_limit,
            auth_header,
            cache_size,
        }
    }
}

impl From<HashMap<&str, &str>> for Config {
    fn from(value: HashMap<&str, &str>) -> Self {
        let oauth = OAuthConfig::from(value.clone());
        let server = ServerConfig::from(value);
        Self { oauth, server }
    }
}

#[derive(Clone)]
pub struct WhiteListKeyExtractor;

impl KeyExtractor for WhiteListKeyExtractor {
    type Key = IpAddr;
    type KeyExtractionError = SimpleKeyExtractionError<&'static str>;

    fn extract(&self, req: &ServiceRequest) -> Result<Self::Key, Self::KeyExtractionError> {
        PeerIpKeyExtractor.extract(req)
    }

    fn whitelisted_keys(&self) -> Vec<Self::Key> {
        // In case we want to add more private networks remember that the CIDR notation for
        // 172s is 172.16.0.0/12 and for 192s is 192.168.0.0/16

        "10.0.0.0/8"
            .parse()
            .map(|ip| vec![ip])
            .unwrap_or_else(|_| vec![])
    }
}
