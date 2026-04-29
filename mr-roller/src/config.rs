use crate::game::player::PlayerId;
use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;

/// Application settings loaded from TOML plus idiomatic environment overrides.
///
/// Loading order:
/// 1. Built-in defaults from `Default` impls.
/// 2. `mr-roller.toml` in the current directory, if present.
/// 3. `MR_ROLLER_CONFIG`, if set, as an additional required config file.
/// 4. Environment overrides using `MR_ROLLER__SECTION__KEY` names.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub admin: AdminConfig,
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct AdminConfig {
    pub bootstrap_admin_ids: Vec<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub url: Option<String>,
}

impl Settings {
    pub fn load() -> Result<Self, ConfigError> {
        let mut builder =
            Config::builder().add_source(File::with_name("mr-roller").required(false));

        if let Ok(path) = std::env::var("MR_ROLLER_CONFIG") {
            builder = builder.add_source(File::with_name(&path).required(true));
        }

        builder
            .add_source(
                Environment::with_prefix("MR_ROLLER")
                    .separator("__")
                    .try_parsing(true)
                    .list_separator(",")
                    .with_list_parse_key("admin.bootstrap_admin_ids"),
            )
            .build()?
            .try_deserialize()
    }

    pub fn bootstrap_admin_player_ids(&self) -> Vec<PlayerId> {
        self.admin
            .bootstrap_admin_ids
            .iter()
            .copied()
            .map(PlayerId::new)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_have_no_admins_or_database_url() {
        let settings = Settings::default();
        assert!(settings.admin.bootstrap_admin_ids.is_empty());
        assert!(settings.database.url.is_none());
    }

    #[test]
    fn converts_bootstrap_admin_ids_to_player_ids() {
        let settings = Settings {
            admin: AdminConfig {
                bootstrap_admin_ids: vec![1, 42],
            },
            database: DatabaseConfig::default(),
        };

        assert_eq!(
            settings.bootstrap_admin_player_ids(),
            vec![PlayerId::new(1), PlayerId::new(42)]
        );
    }
}
