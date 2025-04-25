use serde::{Deserialize, Serialize};
use tokio::fs::read_to_string;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub token: String,
    pub prefix: String,
}
impl Config {
    pub async fn read_from_file() -> Config {
        serde_yaml::from_str(&read_to_string("config.yml").await.unwrap()).unwrap()
    }
}
