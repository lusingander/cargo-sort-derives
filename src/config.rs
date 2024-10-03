use std::path::PathBuf;

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
            order: internal_config.order.map(OrderType::into_vec),
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

impl OrderType {
    fn into_vec(self) -> Vec<String> {
        match self {
            OrderType::String(s) => parse_order(s),
            OrderType::Array(ss) => ss,
        }
    }
}

impl Config {
    pub fn load() -> Config {
        load_from(std::env::current_dir().unwrap())
    }
}

fn load_from(path: PathBuf) -> Config {
    let config_file_paths = CONFIG_FILE_NAMES.iter().map(|p| path.join(p)).collect();

    if let Some(config_file_path) = first_exist_path(config_file_paths) {
        let config_file = std::fs::read_to_string(config_file_path).unwrap();
        let interal_config: InternalConfig = toml::from_str(&config_file).unwrap();
        interal_config.into()
    } else {
        Config::default()
    }
}

fn first_exist_path(paths: Vec<PathBuf>) -> Option<PathBuf> {
    paths.into_iter().find(|p| p.exists())
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
