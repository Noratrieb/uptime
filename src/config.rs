use eyre::{Context, Result};
use url::Url;

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub interval_seconds: u64,
    pub websites: Vec<WebsiteConfig>,
    #[serde(default = "default_db_url")]
    pub db_url: String,
}

fn default_db_url() -> String {
    "uptime.db".into()
}

#[derive(Debug, serde::Deserialize)]
pub struct WebsiteConfig {
    pub name: String,
    pub url: Url,
}

pub fn read_config(config_path: &str) -> Result<Config> {
    let config_str = std::fs::read_to_string(config_path)
        .wrap_err_with(|| format!("opening config at '{config_path}'"))?;

    serde_json::from_str(&config_str).wrap_err("reading config file")
}
