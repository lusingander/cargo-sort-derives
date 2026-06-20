use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::util::parse_order;

const CONFIG_FILE_NAMES: &[&str] = &[".sort-derives.toml", "sort-derives.toml"];

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Config {
    pub order: Option<Vec<String>>,
    pub preserve: Option<bool>,
    pub exclude: Option<Vec<String>>,
}

impl From<InternalConfig> for Config {
    fn from(internal_config: InternalConfig) -> Self {
        Config {
            order: internal_config.order.map(Into::into),
            preserve: internal_config.preserve,
            exclude: internal_config.exclude,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct InternalConfig {
    order: Option<OrderType>,
    preserve: Option<bool>,
    exclude: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OrderType {
    String(String),
    Array(Vec<String>),
}

impl From<OrderType> for Vec<String> {
    fn from(order_type: OrderType) -> Self {
        match order_type {
            OrderType::String(s) => parse_order(s),
            OrderType::Array(ss) => ss,
        }
    }
}

impl Config {
    pub fn load(config_file_path: &Option<String>, no_user_config: bool) -> Config {
        let paths = config_file_path
            .as_ref()
            .map(|p| vec![PathBuf::from(p)])
            .unwrap_or_else(|| {
                let base_dir_path = std::env::current_dir().unwrap();
                CONFIG_FILE_NAMES
                    .iter()
                    .map(|p| base_dir_path.join(p))
                    .collect()
            });

        let mut project_config = load_from(paths);

        if !no_user_config {
            if let Some(user_path) = user_config_path() {
                if user_path.exists() {
                    let global_config = match load_config_file(&user_path) {
                        Ok(cfg) => cfg,
                        Err(e) => {
                            eprintln!(
                                "Warning: failed to parse user config at {}: {}",
                                user_path.display(),
                                e,
                            );

                            Config::default()
                        }
                    };

                    project_config = project_config.merge(global_config);
                }
            }
        }

        project_config
    }

    fn merge(mut self, global: Config) -> Config {
        if self.order.is_none() {
            self.order = global.order;
        }
        if self.preserve.is_none() {
            self.preserve = global.preserve;
        }
        if self.exclude.is_none() {
            self.exclude = global.exclude;
        }

        self
    }
}

fn load_from(config_file_paths: Vec<PathBuf>) -> Config {
    if let Some(config_file_path) = first_exist_path(config_file_paths) {
        load_config_file(&config_file_path).unwrap()
    } else {
        Config::default()
    }
}

fn first_exist_path(paths: Vec<PathBuf>) -> Option<PathBuf> {
    paths.into_iter().find(|p| p.exists())
}

fn user_config_path() -> Option<PathBuf> {
    if let Ok(config_home) = std::env::var("XDG_CONFIG_HOME") {
        if !config_home.is_empty() {
            return Some(PathBuf::from(config_home).join("cargo-sort-derives.toml"));
        }
    }

    dirs::home_dir().map(|p| p.join(".config").join("cargo-sort-derives.toml"))
}

fn load_config_file(path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    let config_file = std::fs::read_to_string(path)?;
    let internal_config: InternalConfig = toml::from_str(&config_file)?;

    Ok(internal_config.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_deserialize_empty() {
        let toml = "";
        let expected = Config::default();

        let actual = deserialize_config(toml);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_config_deserialize_order_is_string() {
        let toml = r#"
            order = "A, B, C"
            preserve = true
            exclude = ["D", "E"]
        "#;
        let expected = config(&["A", "B", "C"], true, &["D", "E"]);

        let actual = deserialize_config(toml);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_config_deserialize_order_is_array() {
        let toml = r#"
            order = ["A", "B", "C"]
            preserve = true
            exclude = ["D", "E"]
        "#;
        let expected = config(&["A", "B", "C"], true, &["D", "E"]);

        let actual = deserialize_config(toml);

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_merge_global_fills_missing() {
        let project = Config {
            order: None,
            preserve: Some(true),
            exclude: None,
        };
        let global = Config {
            order: Some(vec!["A".into(), "B".into()]),
            preserve: None,
            exclude: Some(vec!["C".into()]),
        };
        let merged = project.merge(global);

        assert_eq!(merged.order, Some(vec!["A".to_string(), "B".to_string()]));
        assert_eq!(merged.preserve, Some(true));
        assert_eq!(merged.exclude, Some(vec!["C".to_string()]));
    }

    #[test]
    fn test_merge_project_overrides_global() {
        let project = Config {
            order: Some(vec!["X".into(), "Y".into()]),
            preserve: None,
            exclude: None,
        };
        let global = Config {
            order: Some(vec!["A".into(), "B".into()]),
            preserve: Some(false),
            exclude: Some(vec!["C".into()]),
        };
        let merged = project.merge(global);

        assert_eq!(merged.order, Some(vec!["X".to_string(), "Y".to_string()]));
        assert_eq!(merged.preserve, Some(false));
        assert_eq!(merged.exclude, Some(vec!["C".to_string()]));
    }

    #[test]
    fn test_no_user_config_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let cwd = std::env::current_dir().unwrap();

        std::env::set_current_dir(dir.path()).unwrap();

        let config = Config::load(&None, true);
        assert_eq!(config, Config::default());

        std::env::set_current_dir(cwd).unwrap();
    }

    #[test]
    fn test_malformed_global_warns() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("bad-config.toml");
        std::fs::write(&config_path, "this is not valid toml = [").unwrap();

        let result = load_config_file(&config_path);
        assert!(
            result.is_err(),
            "load_config_file should return Err for malformed TOML"
        );

        let config = match result {
            Ok(cfg) => cfg,
            Err(_) => Config::default(),
        };
        assert_eq!(config, Config::default());
    }

    fn config(order: &[&str], preserve: bool, exclude: &[&str]) -> Config {
        Config {
            order: Some(order.iter().map(|s| s.to_string()).collect()),
            preserve: Some(preserve),
            exclude: Some(exclude.iter().map(|s| s.to_string()).collect()),
        }
    }

    fn deserialize_config(toml: &str) -> Config {
        let interal_config: InternalConfig = toml::from_str(toml).unwrap();
        interal_config.into()
    }
}
