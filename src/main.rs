#![allow(dead_code, unused)]

mod aws;
mod cli;
mod config;
mod scenarios;
mod ssh;

#[tokio::main]
pub async fn main() {
    cli::run().await
}
