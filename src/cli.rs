use clap::{Parser, Subcommand};

use crate::aws::scenarios::{AWSNetwork, Swarm};
use crate::aws::{ec2, tagged};
use crate::config::SwarmConfig;
use crate::ssh::SSHPool;

const ABOUT_CLI: &str = "killabeez: a CLI for creating traffic jams of arbitrary scale";
const DEFAULT_CONFIG: &str = "swarm.toml";

#[derive(Debug, Parser)]
#[command(version)]
#[command(about = ABOUT_CLI)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Init {
        #[arg(short, long, value_name = "FILE")]
        config: Option<String>,
    },
    Tagged {
        #[arg(short, long, value_name = "FILE")]
        config: Option<String>,
    },
    Terminate {
        #[arg(short, long, value_name = "FILE")]
        config: Option<String>,
    },
    Exec {
        #[arg(short, long, value_name = "FILE")]
        config: Option<String>,
    },
}

pub fn config_or_default(config: Option<String>) -> String {
    match config {
        Some(filename) => filename.clone(),
        None => DEFAULT_CONFIG.to_string(),
    }
}

pub async fn run() {
    let args = Cli::parse();

    let client = ec2::mk_client().await;

    match args.command {
        Commands::Init { config } => {
            println!("[cli init]");
            let sc = SwarmConfig::read(&config_or_default(config)).unwrap();
            println!("{}", sc);

            let network = AWSNetwork::init_network(&client, &sc).await.unwrap();
            let swarm = Swarm::init_swarm(&client, &sc, &network).await.unwrap();
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

            let hosts = swarm
                .instances
                .iter()
                .map(|i| i.ip.clone().unwrap())
                .collect::<Vec<String>>();

            let auth = SSHPool::load_key(&sc).unwrap();
            let ssh_pool = SSHPool::new(&hosts, &sc.username.unwrap(), &auth).await;
            let results = ssh_pool.exec("ls").await;
            crate::ssh::print_results(results)
        }
    }
}
