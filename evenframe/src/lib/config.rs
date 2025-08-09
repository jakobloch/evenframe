use serde::{Deserialize, Serialize};
use std::{env, fs, path::Path};
use toml;

/// Unified configuration for Evenframe operations
/// This is the root configuration that contains both schemasync and typesync configurations
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvenframeConfig {
    /// Schema synchronization configuration (database operations)
    pub schemasync: crate::schemasync::config::SchemasyncConfig,
    /// Type synchronization configuration (TypeScript/Effect type generation)
    pub typesync: crate::typesync::config::TypesyncConfig,
}

impl EvenframeConfig {
    /// Load configuration from evenframe.toml
    pub fn new() -> Result<EvenframeConfig, Box<dyn std::error::Error>> {
        dotenv::dotenv().ok();

        let config_path_string = format!(
            "{}{}",
            env::var("ABSOLUTE_PATH").expect("ABSOLUTE_PATH is not set"),
            "/backend/evenframe.toml"
        );
        // Try to find evenframe.toml in the backend directory
        let config_path = Path::new(&config_path_string);

        if !config_path.exists() {
            return Err("evenframe.toml not found. Configuration file is required.".into());
        }

        let contents = fs::read_to_string(config_path)?;
        let mut config: EvenframeConfig = toml::from_str(&contents)?;

        // Process environment variable substitutions for all database-related fields
        config.schemasync.database.url = Self::substitute_env_vars(&config.schemasync.database.url);
        config.schemasync.database.namespace =
            Self::substitute_env_vars(&config.schemasync.database.namespace);
        config.schemasync.database.database =
            Self::substitute_env_vars(&config.schemasync.database.database);

        Ok(config)
    }

    /// Substitute environment variables in config strings
    /// Supports ${VAR_NAME:-default} syntax
    fn substitute_env_vars(value: &str) -> String {
        let mut result = value.to_string();

        // Pattern to match ${VAR_NAME} or ${VAR_NAME:-default}
        let re = regex::Regex::new(r"\$\{([^}:]+)(?::-([^}]*))?\}")
            .expect("There were no matches for the given environment variables");

        for cap in re.captures_iter(value) {
            let var_name = &cap[1];
            let _default_value = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            let replacement = env::var(var_name).expect(&format!("{} was not set", var_name));
            let full_match = &cap[0];
            result = result.replace(full_match, &replacement);
        }

        result
    }
}
