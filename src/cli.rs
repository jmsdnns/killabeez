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
    #[command(arg_required_else_help = true)]
    Init {
        #[arg(required = true)]
        name: String,

        #[arg(default_value_t = 1)]
        count: i32,
    },

    Tagged {
        #[arg(required = true)]
        name: String,
    },

    #[command(arg_required_else_help = true)]
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

pub async fn run(sc: &SwarmConfig) {
    let args = Cli::parse();

    let Ok(client) = ec2::mk_client().await else {
        panic!("[cli] error: mk_client");
    };

    match args.command {
        Commands::Init { name, count } => {
            println!("[cli init] {name}");
            let network = AWSNetwork::load_network(&client, sc).await.unwrap();
            let swarm = Swarm::load_swarm(&client, sc, &network).await.unwrap();
            println!("#####################");
            println!("VPC ID:    {}", network.vpc_id);
            println!("Subnet ID: {}", network.subnet_id);
            println!("SG ID:     {}", network.security_group_id);
            println!("SSH Key:   {}", swarm.key_pair);
            println!(
                "Instances: {}",
                swarm
                    .instances
                    .iter()
                    .map(|b| b.ip.clone().unwrap())
                    .collect::<Vec<String>>()
                    .join(", ")
            );
        }
        Commands::Tagged { name } => {
            println!("[cli tagged] {name}");
            tagged::all_beez_tags().await;
        }
        Commands::Terminate { name } => {
            println!("[cli terminate] {name}");
            Swarm::drop_swarm(&client, sc).await;
            AWSNetwork::drop_network(&client, sc).await;
        }
        Commands::Exec { name, script } => {
            println!("[cli exec] {name} {:?}", script);
            let network = AWSNetwork::load_network(&client, sc).await.unwrap();
            let swarm = Swarm::load_swarm(&client, sc, &network).await.unwrap();
        }
    }
}
