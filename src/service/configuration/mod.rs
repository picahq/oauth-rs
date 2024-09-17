use envconfig::Envconfig;
use integrationos_domain::{
    database::DatabaseConfig, environment::Environment, secrets::SecretsConfig,
};
use std::fmt::Debug;

#[derive(Clone, Envconfig)]
pub struct RefreshConfig {
    #[envconfig(from = "REFRESH_BEFORE_IN_MINUTES", default = "10")]
    refresh_before: i64,
    #[envconfig(from = "SLEEP_TIMER_IN_SECONDS", default = "20")]
    sleep_timer: u64,
    #[envconfig(nested = true)]
    database: DatabaseConfig,
    #[envconfig(nested = true)]
    secrets_config: SecretsConfig,
    #[envconfig(from = "TIMEOUT", default = "30")]
    timeout: u64,
    #[envconfig(from = "ENVIRONMENT", default = "test")]
    environment: Environment,
    #[envconfig(from = "GET_SECRET_PATH", default = "http://localhost:3005/v1/secrets")]
    get_secret: String,
    #[envconfig(
        from = "CREATE_SECRET_PATH",
        default = "http://localhost:3005/v1/secrets"
    )]
    create_secret: String,
    #[envconfig(from = "MAX_RETRIES", default = "3")]
    max_retries: u32,
}

impl Debug for RefreshConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "REFRESH_BEFORE_IN_MINUTES: {}", self.refresh_before)?;
        writeln!(f, "SLEEP_TIMER_IN_SECONDS: {}", self.sleep_timer)?;
        writeln!(f, "TIMEOUT: {}", self.timeout)?;
        writeln!(f, "ENVIRONMENT: {}", self.environment)?;
        writeln!(f, "GET_SECRET_PATH: {}", self.get_secret)?;
        writeln!(f, "CREATE_SECRET_PATH: {}", self.create_secret)?;
        writeln!(f, "MAX_RETRIES: {}", self.max_retries)?;
        write!(f, "{}", self.database)?;
        write!(f, "{}", self.secrets_config)
    }
}

impl RefreshConfig {
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

    pub fn timeout(&self) -> u64 {
        self.timeout
    }

    pub fn environment(&self) -> Environment {
        self.environment
    }

    pub fn get_secret(&self) -> &str {
        &self.get_secret
    }

    pub fn create_secret(&self) -> &str {
        &self.create_secret
    }

    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }
}
