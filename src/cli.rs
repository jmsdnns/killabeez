use clap::{Parser, Subcommand};

use crate::aws::scenarios::{AWSNetwork, Swarm};
use crate::aws::{ec2, tagged};
use crate::config::SwarmConfig;
use crate::ssh::errors::SshError;
use crate::ssh::pools::SSHPool;

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
    Upload {
        #[arg(short, long, value_name = "FILE")]
        config: Option<String>,

        #[arg(required = true)]
        //filenames: Vec<String>,
        filename: String,
    },
    Download {
        #[arg(short, long, value_name = "FILE")]
        config: Option<String>,

        #[arg(required = true)]
        //filenames: Vec<String>,
        filename: String,
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

            let network = AWSNetwork::init(&client, &sc).await.unwrap();
            let swarm = Swarm::init(&client, &sc, &network).await.unwrap();
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

            Swarm::drop(&client, &sc).await;
            AWSNetwork::drop(&client, &sc).await;
        }
        Commands::Exec { config } => {
            println!("[cli exec]");
            let sc = SwarmConfig::read(&config_or_default(config)).unwrap();
            println!("{}", sc);

            let network = AWSNetwork::load(&client, &sc).await.unwrap();
            let swarm = Swarm::load(&client, &sc, &network).await.unwrap();
            println!("{}", swarm);

            let hosts = swarm
                .instances
                .iter()
                .map(|i| i.ip.clone().unwrap())
                .collect::<Vec<String>>();

            let auth = SSHPool::load_key(&sc).unwrap();
            let ssh_pool = SSHPool::new(&hosts, &sc.username.unwrap(), auth).await;
            let results = ssh_pool.execute("ls").await;
            for r in results.iter() {
                println!("{}", r.as_ref().unwrap());
            }
        }
        Commands::Upload { config, filename } => {
            println!("[cli upload]");
            let sc = SwarmConfig::read(&config_or_default(config)).unwrap();
            println!("{}", sc);

            let network = AWSNetwork::load(&client, &sc).await.unwrap();
            let swarm = Swarm::load(&client, &sc, &network).await.unwrap();
            println!("{}", swarm);

            let hosts = swarm
                .instances
                .iter()
                .map(|i| i.ip.clone().unwrap())
                .collect::<Vec<String>>();

            let auth = SSHPool::load_key(&sc).unwrap();
            let ssh_pool = SSHPool::new(&hosts, &sc.username.unwrap(), auth).await;
            let results = ssh_pool.upload(&filename).await;
            for r in results.iter() {
                println!("{}", r.as_ref().unwrap());
            }
        }
        Commands::Download { config, filename } => {
            println!("[cli download]");
            let sc = SwarmConfig::read(&config_or_default(config)).unwrap();
            println!("{}", sc);

            let network = AWSNetwork::load(&client, &sc).await.unwrap();
            let swarm = Swarm::load(&client, &sc, &network).await.unwrap();
            println!("{}", swarm);

            let hosts = swarm
                .instances
                .iter()
                .map(|i| i.ip.clone().unwrap())
                .collect::<Vec<String>>();

            let auth = SSHPool::load_key(&sc).unwrap();
            let ssh_pool = SSHPool::new(&hosts, &sc.username.unwrap(), auth).await;
            let results = ssh_pool.download(&filename).await;
            for r in results.iter() {
                println!("{}", r.as_ref().unwrap());
            }
        }
    }
}
