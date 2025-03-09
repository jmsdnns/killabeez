use std::{env, fmt};

use figment::{
    Figment,
    error::Error,
    providers::{Env, Format, Toml},
};
use serde::{self, Deserialize};

const DEFAULT_SSH_CIDR: &str = "0.0.0.0/0";
const DEFAULT_AMI: &str = "ami-04b4f1a9cf54c11d0";
const DEFAULT_USERNAME: &str = "ubuntu";

#[derive(Debug, Clone, Deserialize)]
pub struct SwarmConfig {
    pub tag_name: String,
    pub num_beez: i32,
    pub ssh_cidr_block: Option<String>,
    pub username: Option<String>,
    pub ami: Option<String>,
    pub public_key_file: Option<String>,
    pub key_id: Option<String>,
    pub vpc_id: Option<String>,
    pub subnet_id: Option<String>,
    pub security_group_id: Option<String>,
}

impl fmt::Display for SwarmConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "CONFIG ]---------------------------\n\
             Tag Name:     {}\n\
             Num Beez:     {}\n\
             SSH CIDR:     {}\n\
             Username:     {}\n\
             AMI:          {}\n\
             Pub Key File: {}\n\
             Key Id:       {}\n\
             VPC Id:       {}\n\
             Subnet Id:    {}\n\
             Sec Group Id: {}",
            self.tag_name,
            self.num_beez,
            self.ssh_cidr_block.clone().unwrap_or("none".to_string()),
            self.username.clone().unwrap_or("none".to_string()),
            self.ami.clone().unwrap_or("none".to_string()),
            self.public_key_file.clone().unwrap_or("none".to_string()),
            self.key_id.clone().unwrap_or("none".to_string()),
            self.vpc_id.clone().unwrap_or("none".to_string()),
            self.subnet_id.clone().unwrap_or("none".to_string()),
            self.security_group_id.clone().unwrap_or("none".to_string()),
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

        sc.username = match &sc.username {
            None => Some(DEFAULT_USERNAME.to_string()),
            Some(username) => Some(username.to_string()),
        };

        sc.ami = match &sc.ami {
            None => Some(DEFAULT_AMI.to_string()),
            Some(ami) => Some(ami.to_string()),
        };

        if sc.public_key_file.is_none() && sc.key_id.is_none() {
            panic!("ERROR: swarm config must contain a public_key_file or a key_id");
        };

        Ok(sc)
    }

    pub fn private_key_file(&self) -> Option<String> {
        match self.public_key_file.clone() {
            Some(mut pkf) => match pkf.ends_with(".pub") {
                true => Some(pkf[..pkf.len() - 4].to_string()),
                false => unimplemented!(),
            },
            None => None,
        }
    }
}
