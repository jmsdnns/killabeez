use figment::error::Error;
use serde::{self, Deserialize};
use std::env;

use figment::{
    Figment,
    providers::{Env, Format, Toml},
};

const DEFAULT_SSH_CIDR: &str = "0.0.0.0/0";
const DEFAULT_AMI: &str = "ami-0c55b159cbfafe1f0";

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub username: String,
    pub key_file: String,
    pub tag_name: String,
    pub num_beez: i32,
    pub vpc_id: Option<String>,
    pub ssh_cidr_block: Option<String>,
    pub subnet_id: Option<String>,
    pub security_group_id: Option<String>,
    pub ami: Option<String>,
}

impl AppConfig {
    pub fn read(filename: &str) -> Result<Self, Error> {
        let mut ac: AppConfig = Figment::new().merge(Toml::file(filename)).extract()?;

        let ssh_cidr_block = match &ac.ssh_cidr_block {
            None => DEFAULT_SSH_CIDR.to_string(),
            Some(cb) => cb.clone(),
        };
        ac.ssh_cidr_block = Some(ssh_cidr_block);

        let ami = match &ac.ami {
            None => DEFAULT_AMI.to_string(),
            Some(ami) => ami.to_string(),
        };
        ac.ami = Some(ami);

        Ok(ac)
    }
}
