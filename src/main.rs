#![allow(dead_code, unused)]

pub mod aws;
mod cli;
mod config;
mod scenarios;

#[tokio::main]
pub async fn main() {
    cli::run().await
}
