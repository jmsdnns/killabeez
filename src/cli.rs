use clap::{Parser, Subcommand};

use crate::aws::{ec2, tagged};
use crate::config::SwarmConfig;
use crate::scenarios::{AWSNetwork, Swarm};

#[derive(Debug, Parser)]
#[command(about = "A CLI for killabeez", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init { config: Option<String> },
    Tagged { config: Option<String> },
    Terminate { config: Option<String> },
    Exec { config: Option<String> },
}

pub fn config_or_default(config: Option<String>) -> String {
    match config {
        Some(filename) => filename.clone(),
        None => "sshpools.toml".to_string(),
    }
}

pub async fn run() {
    let args = Cli::parse();

    let Ok(client) = ec2::mk_client().await else {
        panic!("[cli] error: mk_client");
    };

    match args.command {
        Commands::Init { config } => {
            println!("[cli init]");
            let sc = SwarmConfig::read(&config_or_default(config)).unwrap();
            println!("{}", sc);

            let network = AWSNetwork::load_network(&client, &sc).await.unwrap();
            let swarm = Swarm::load_swarm(&client, &sc, &network).await.unwrap();
            println!("{}", swarm);
        }
        Commands::Tagged { config } => {
            println!("[cli tagged]");
            let sc = SwarmConfig::read(&config_or_default(config)).unwrap();

            tagged::all_beez_tags().await;
        }
        Commands::Terminate { config } => {
            println!("[cli terminate]");
            let sc = SwarmConfig::read(&config_or_default(config)).unwrap();
            println!("{}", sc);

            Swarm::drop_swarm(&client, &sc).await;
            AWSNetwork::drop_network(&client, &sc).await;
        }
        Commands::Exec { config } => {
            println!("[cli exec]");
            let sc = SwarmConfig::read(&config_or_default(config)).unwrap();
            println!("{}", sc);

            let network = AWSNetwork::load_network(&client, &sc).await.unwrap();
            let swarm = Swarm::load_swarm(&client, &sc, &network).await.unwrap();
            println!("{}", swarm);
        }
    }
}
