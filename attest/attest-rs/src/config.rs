use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AttestConfig {
    pub database_path: String,
    pub identity_path: Option<String>,
    pub watch_paths: Vec<String>,
}

impl AttestConfig {
    pub fn new() -> Result<Self, ConfigError> {
        let home = dirs::home_dir().expect("Could not determine home directory");
        let default_db = home.join(".attest/attest.db").to_string_lossy().to_string();
        let default_id = home
            .join(".attest/identity.enc")
            .to_string_lossy()
            .to_string();

        let s = Config::builder()
            // 1. Start with defaults
            .set_default("database_path", default_db)?
            .set_default("identity_path", default_id)?
            .set_default("watch_paths", Vec::<String>::new())?
            // 2. Load from config file (if exists)
            .add_source(
                File::with_name(&home.join(".attest/config").to_string_lossy()).required(false),
            )
            // 3. Load from Environment Variables (ATTEST_DATABASE_PATH, etc.)
            .add_source(Environment::with_prefix("ATTEST"))
            .build()?;

        s.try_deserialize()
    }
}
