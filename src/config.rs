use std::{env, fmt};

use figment::{
    Figment,
    error::Error,
    providers::{Env, Format, Toml},
};
use serde::{self, Deserialize};

const DEFAULT_SSH_CIDR: &str = "0.0.0.0/0";
const DEFAULT_AMI: &str = "ami-04b4f1a9cf54c11d0";

#[derive(Debug, Clone, Deserialize)]
pub struct SwarmConfig {
    pub tag_name: String,
    pub num_beez: i32,
    pub username: String,
    pub key_file: Option<String>,
    pub key_id: Option<String>,
    pub ssh_cidr_block: Option<String>,
    pub vpc_id: Option<String>,
    pub subnet_id: Option<String>,
    pub security_group_id: Option<String>,
    pub ami: Option<String>,
}

impl fmt::Display for SwarmConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "CONFIG ]---------------------------\n\
             Tag Name:     {}\n\
             Num Beez:     {}\n\
             Username:     {}\n\
             Key File:     {}\n\
             Key Id:       {}\n\
             SSH CIDR:     {}\n\
             VPC Id:       {}\n\
             Subnet Id:    {}\n\
             Sec Group Id: {}\n\
             AMI:          {}",
            self.tag_name,
            self.num_beez,
            self.username,
            self.key_file.clone().unwrap_or_default(),
            self.key_id.clone().unwrap_or_default(),
            self.ssh_cidr_block.clone().unwrap_or_default(),
            self.vpc_id.clone().unwrap_or_default(),
            self.subnet_id.clone().unwrap_or_default(),
            self.security_group_id.clone().unwrap_or_default(),
            self.ami.clone().unwrap_or_default(),
        )
    }
}

impl SwarmConfig {
    pub fn read(filename: &str) -> Result<Self, Error> {
        let mut sc: SwarmConfig = Figment::new().merge(Toml::file(filename)).extract()?;

        sc.ssh_cidr_block = match &sc.ssh_cidr_block {
            None => Some(DEFAULT_SSH_CIDR.to_string()),
            Some(cb) => Some(cb.clone()),
        };

        sc.ami = match &sc.ami {
            None => Some(DEFAULT_AMI.to_string()),
            Some(ami) => Some(ami.to_string()),
        };

        Ok(sc)
    }
}
