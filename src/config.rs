use figment::error::Error;
use serde::{self, Deserialize};
use std::env;

use figment::{
    Figment,
    providers::{Env, Format, Toml},
};

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub username: String,
    pub key_file: String,
    pub tag_name: String,
    pub num_beez: i32,
    pub cidr_block: Option<String>,
}

impl AppConfig {
    pub fn read(filename: &str) -> Result<Self, Error> {
        let mut ac: AppConfig = Figment::new().merge(Toml::file(filename)).extract()?;

        // TODO kinda risky
        let cidr_block = match &ac.cidr_block {
            None => "0.0.0.0/24".to_string(),
            Some(cb) => cb.clone(),
        };
        ac.cidr_block = Some(cidr_block);

        Ok(ac)
    }
}
