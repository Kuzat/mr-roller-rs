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
    pub events: EventsConfig,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct EventsConfig {
    pub enabled: bool,
    pub check_interval_seconds: u64,
    pub spawn_chance_per_check: f64,
    pub max_active_events: usize,
    pub random_item_spawn: RandomItemSpawnConfig,
}

impl Default for EventsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval_seconds: 60,
            spawn_chance_per_check: 0.004,
            max_active_events: 1,
            random_item_spawn: RandomItemSpawnConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RandomItemSpawnConfig {
    pub enabled: bool,
    pub timeout_seconds: u64,
    pub items: Vec<WeightedEventItemConfig>,
}

impl Default for RandomItemSpawnConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            timeout_seconds: 900,
            items: vec![
                WeightedEventItemConfig {
                    kind: "regular_dice".to_string(),
                    weight: 5,
                },
                WeightedEventItemConfig {
                    kind: "lucky_dice".to_string(),
                    weight: 1,
                },
                WeightedEventItemConfig {
                    kind: "cursed_dice".to_string(),
                    weight: 3,
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WeightedEventItemConfig {
    pub kind: String,
    pub weight: u32,
}

impl Default for WeightedEventItemConfig {
    fn default() -> Self {
        Self {
            kind: "regular_dice".to_string(),
            weight: 1,
        }
    }
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
        assert_eq!(settings.events.check_interval_seconds, 60);
    }

    #[test]
    fn converts_bootstrap_admin_ids_to_player_ids() {
        let settings = Settings {
            admin: AdminConfig {
                bootstrap_admin_ids: vec![1, 42],
            },
            database: DatabaseConfig::default(),
            events: EventsConfig::default(),
        };

        assert_eq!(
            settings.bootstrap_admin_player_ids(),
            vec![PlayerId::new(1), PlayerId::new(42)]
        );
    }
}
