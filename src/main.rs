#![allow(dead_code, unused)]

mod aws;
mod beez;
mod cli;
mod config;

#[tokio::main]
pub async fn main() {
    let config_file = "sshpools.toml";

    let Ok(cfg) = config::AppConfig::read(config_file) else {
        panic!("Booooo");
    };
    println!("CONFIG:");
    println!("- username: {}", cfg.username);
    println!("- key file: {}", cfg.key_file);

    cli::run(cfg).await
}
