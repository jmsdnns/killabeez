#![allow(dead_code, unused)]

pub mod aws;
mod cli;
mod config;
mod scenarios;

#[tokio::main]
pub async fn main() {
    let config_file = "sshpools.toml";

    let Ok(sc) = config::SwarmConfig::read(config_file) else {
        panic!("Booooo");
    };

    cli::run(&sc).await
}
