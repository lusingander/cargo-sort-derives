use serde::Deserialize;

const CONFIG_FILE_NAME: &str = ".sort-derives.toml";

#[derive(Debug, Default, Deserialize)]
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

fn parse_order(order: String) -> Vec<String> {
    order
        .split(',')
        .map(str::trim)
        .map(str::to_string)
        .collect()
}

impl Config {
    pub fn load() -> Config {
        let config_file_path = std::env::current_dir().unwrap().join(CONFIG_FILE_NAME);

        if config_file_path.exists() {
            let config_file = std::fs::read_to_string(&config_file_path).unwrap();
            let interal_config: InternalConfig = toml::from_str(&config_file).unwrap();
            interal_config.into()
        } else {
            Config::default()
        }
    }
}
