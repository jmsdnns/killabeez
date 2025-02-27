#![allow(dead_code, unused)]

mod aws;
mod beez;
mod cli;
mod config;

#[tokio::main]
pub async fn main() {
    let config_file = "sshpools.toml";

    let Ok(ac) = config::AppConfig::read(config_file) else {
        panic!("Booooo");
    };

    cli::run(ac).await
}
