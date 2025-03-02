use clap::{Parser, Subcommand};

use crate::aws::{ec2, tagged};
use crate::config::AppConfig;
use crate::scenarios::{AWSNetwork, Swarm};

#[derive(Debug, Parser)]
#[command(about = "A CLI for killabeez", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Init {
        #[arg(required = true)]
        name: String,

        #[arg(default_value_t = 1)]
        count: i32,
    },

    Network {
        #[arg(required = true)]
        name: String,
    },

    Tagged {
        name: String,
    },

    Terminate {
        #[arg(required = true)]
        name: String,
    },

    Exec {
        #[arg(required = true)]
        name: String,

        #[arg(short = 's')]
        script: Option<String>,
    },
}

pub async fn run(ac: &AppConfig) {
    let args = Cli::parse();

    let Ok(client) = ec2::mk_client(ac).await else {
        panic!("[cli] error: mk_client");
    };

    match args.command {
        Commands::Init { name, count } => {
            println!("[cli init] {name}");
            let network = AWSNetwork::load_network(&client, ac).await.unwrap();
            let swarm = Swarm::init_swarm(&client, ac, &network).await.unwrap();
        }
        Commands::Network { name } => {
            println!("[cli network] {name}");
            AWSNetwork::load_network(&client, ac).await;
        }
        Commands::Tagged { name } => {
            println!("[cli tagged] {name}");
            tagged::all_beez_tags().await;
        }
        Commands::Terminate { name } => {
            println!("[cli terminate] {name}");
        }
        Commands::Exec { name, script } => {
            println!("[cli exec] {name} {:?}", script);
            let network = AWSNetwork::load_network(&client, ac).await.unwrap();
            let swarm = Swarm::load_swarm(&client, ac, &network).await.unwrap();
        }
    }
}
