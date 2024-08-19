use serde::Deserialize;

const CONFIG_FILE_NAME: &str = ".sort-derives.toml";

#[derive(Debug, Default, Deserialize)]
pub struct Config {
    pub order: Option<String>,
}

impl Config {
    pub fn load() -> Config {
        let config_file_path = std::env::current_dir().unwrap().join(CONFIG_FILE_NAME);

        if config_file_path.exists() {
            let config_file = std::fs::read_to_string(&config_file_path).unwrap();
            toml::from_str(&config_file).unwrap()
        } else {
            Config::default()
        }
    }
}
