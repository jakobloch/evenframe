use crate::error::{EvenframeError, Result};
use serde::{Deserialize, Serialize};
use std::{env, fs, path::Path};
use toml;
use tracing::{debug, error, info, trace, warn};

/// General configuration for Evenframe operations
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct GeneralConfig {
    /// Attribute macro names that expand to include Evenframe derive
    /// These are used with #[apply(...)] and automatically include Evenframe
    #[serde(default)]
    pub apply_aliases: Vec<String>,
}

/// Unified configuration for Evenframe operations
/// This is the root configuration that contains both schemasync and typesync configurations
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvenframeConfig {
    /// General configuration
    #[serde(default)]
    pub general: GeneralConfig,
    /// Schema synchronization configuration (database operations)
    pub schemasync: crate::schemasync::config::SchemasyncConfig,
    /// Type synchronization configuration (TypeScript/Effect type generation)
    pub typesync: crate::typesync::config::TypesyncConfig,
}

impl EvenframeConfig {
    /// Load configuration from evenframe.toml
    pub fn new() -> Result<EvenframeConfig> {
        info!("Loading Evenframe configuration");
        dotenv::dotenv().ok();
        debug!("Environment variables loaded from .env");

        let absolute_path = env::var("ABSOLUTE_PATH").map_err(|e| {
            error!("ABSOLUTE_PATH environment variable not set: {}", e);
            EvenframeError::EnvVarNotSet("ABSOLUTE_PATH".to_string())
        })?;

        let config_path_string = format!("{}{}", absolute_path, "/backend/evenframe.toml");
        debug!("Looking for config file at: {}", config_path_string);

        // Try to find evenframe.toml in the backend directory
        let config_path = Path::new(&config_path_string);

        if !config_path.exists() {
            error!("Configuration file not found at: {:?}", config_path);
            return Err(EvenframeError::config(
                "evenframe.toml not found. Configuration file is required.",
            ));
        }
        info!("Found configuration file at: {:?}", config_path);

        let contents = fs::read_to_string(config_path).map_err(|e| {
            error!("Failed to read configuration file: {}", e);
            e
        })?;

        debug!("Configuration file size: {} bytes", contents.len());

        let mut config: EvenframeConfig = toml::from_str(&contents).map_err(|e| {
            error!("Failed to parse TOML configuration: {}", e);
            e
        })?;

        debug!("Successfully parsed TOML configuration");

        // Process environment variable substitutions for all database-related fields
        debug!("Substituting environment variables in configuration");
        config.schemasync.database.url = Self::substitute_env_vars(&config.schemasync.database.url);
        config.schemasync.database.namespace =
            Self::substitute_env_vars(&config.schemasync.database.namespace);
        config.schemasync.database.database =
            Self::substitute_env_vars(&config.schemasync.database.database);

        info!("Configuration loaded successfully");
        debug!(
            "Schemasync enabled: {}, Typesync arktype: {}, effect: {}",
            config.schemasync.should_generate_mocks,
            config.typesync.should_generate_arktype_types,
            config.typesync.should_generate_effect_types
        );

        Ok(config)
    }

    /// Substitute environment variables in config strings
    /// Supports ${VAR_NAME:-default} syntax
    fn substitute_env_vars(value: &str) -> String {
        trace!("Substituting environment variables in: {}", value);
        let mut result = value.to_string();

        // Pattern to match ${VAR_NAME} or ${VAR_NAME:-default}
        let re = regex::Regex::new(r"\$\{([^}:]+)(?::-([^}]*))?\}")
            .expect("There were no matches for the given environment variables");

        for cap in re.captures_iter(value) {
            let var_name = &cap[1];
            let default_value = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            trace!("Looking for environment variable: {}", var_name);

            let replacement = env::var(var_name)
                .inspect(|_| {
                    if default_value.is_empty() {
                        error!(
                            "Environment variable {} not set and no default provided",
                            var_name
                        );
                    } else {
                        warn!(
                            "Environment variable {} not set, using default: {}",
                            var_name, default_value
                        );
                    }
                })
                .unwrap_or_else(|_| panic!("{var_name} was not set"));

            let full_match = &cap[0];
            debug!("Replacing {} with value from {}", full_match, var_name);
            result = result.replace(full_match, &replacement);
        }

        result
    }
}
