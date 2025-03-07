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
    Init {
        //config: String,
    },

    Tagged {
        //config: String,
    },

    Terminate {
        //config: String,
    },

    Exec {
        //config: String,
    },
}

pub async fn run(sc: &SwarmConfig) {
    let args = Cli::parse();

    let Ok(client) = ec2::mk_client().await else {
        panic!("[cli] error: mk_client");
    };

    match args.command {
        Commands::Init {} => {
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
        Commands::Tagged {} => {
            println!("[cli tagged]");
            tagged::all_beez_tags().await;
        }
        Commands::Terminate {} => {
            println!("[cli terminate]");
            Swarm::drop_swarm(&client, sc).await;
            AWSNetwork::drop_network(&client, sc).await;
        }
        Commands::Exec {} => {
            println!("[cli exec]");
            let network = AWSNetwork::load_network(&client, sc).await.unwrap();
            let swarm = Swarm::load_swarm(&client, sc, &network).await.unwrap();
        }
    }
}
