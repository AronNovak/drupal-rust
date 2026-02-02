use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub site: SiteConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SiteConfig {
    pub name: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Config {
            server: ServerConfig {
                host: env::var("DRUPAL_SERVER__HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
                port: env::var("DRUPAL_SERVER__PORT")
                    .unwrap_or_else(|_| "8080".to_string())
                    .parse()
                    .map_err(|_| ConfigError::InvalidPort)?,
            },
            database: DatabaseConfig {
                url: env::var("DRUPAL_DATABASE__URL")
                    .map_err(|_| ConfigError::MissingDatabaseUrl)?,
            },
            site: SiteConfig {
                name: env::var("DRUPAL_SITE__NAME").unwrap_or_else(|_| "Drupal".to_string()),
            },
        })
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("DRUPAL_DATABASE__URL environment variable is required")]
    MissingDatabaseUrl,
    #[error("Invalid port number")]
    InvalidPort,
}
