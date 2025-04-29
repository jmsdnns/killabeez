use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

use crate::aws::scenarios::{AWSNetwork, Swarm};
use crate::aws::{ec2, tagged};
use crate::config::SwarmConfig;
use crate::ssh::errors::SshError;
use crate::ssh::files::DEFAULT_LOCAL_ROOT;
use crate::ssh::io::IOConfig;
use crate::ssh::pools::SSHPool;
use aws_sdk_ec2::Client;

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

        #[arg(short, long, default_value_t = DEFAULT_LOCAL_ROOT.to_string())]
        datadir: String,

        #[arg(short, long, default_value_t = false)]
        verbose: bool,

        #[arg(short, long, default_value_t = false)]
        stream: bool,
    },

    Upload {
        #[arg(short, long, value_name = "FILE")]
        config: Option<String>,

        #[arg(short, long, default_value_t = DEFAULT_LOCAL_ROOT.to_string())]
        datadir: String,

        #[arg(short, long, default_value_t = false)]
        verbose: bool,

        #[arg(short, long, default_value_t = false)]
        stream: bool,

        #[arg(required = true)]
        filename: String,
    },

    Download {
        #[arg(short, long, value_name = "FILE")]
        config: Option<String>,

        #[arg(short, long, default_value_t = DEFAULT_LOCAL_ROOT.to_string())]
        datadir: String,

        #[arg(short, long, default_value_t = false)]
        verbose: bool,

        #[arg(short, long, default_value_t = false)]
        stream: bool,

        #[arg(required = true)]
        filename: String,
    },
}

pub fn config_or_default(config: Option<String>) -> String {
    match config {
        Some(filename) => filename.clone(),
        None => DEFAULT_CONFIG.to_string(),
    }
}

async fn load_ssh_pool(
    client: &Client,
    config: Option<String>,
    datadir: String,
    verbose: bool,
    stream: bool,
) -> SSHPool {
    let sc = SwarmConfig::read(&config_or_default(config)).unwrap();
    let network = AWSNetwork::load(client, &sc).await.unwrap();
    let swarm = Swarm::load(client, &sc, &network).await.unwrap();
    println!("{}", sc);
    println!("{}", swarm);

    let hosts = swarm
        .instances
        .iter()
        .map(|i| i.ip.clone().unwrap())
        .collect::<Vec<String>>();

    let auth = SSHPool::load_key(&sc).unwrap();

    let io_config = match stream {
        true => IOConfig::Stream(PathBuf::from(datadir.clone()), verbose),
        false => IOConfig::Remote(PathBuf::from(datadir.clone()), None, verbose),
    };

    SSHPool::new(&hosts, &sc.username.unwrap(), auth, io_config).await
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

        Commands::Exec {
            config,
            datadir,
            verbose,
            stream,
        } => {
            println!("[cli exec]");

            let ssh_pool = load_ssh_pool(&client, config, datadir, verbose, stream).await;

            // NOTE: will become flexible soon
            ssh_pool.execute("hostname").await;
            ssh_pool.execute("ls -la").await;

            println!("[cli exec] fetching remote artifacts");
            ssh_pool.finish().await;
        }

        Commands::Upload {
            config,
            datadir,
            verbose,
            stream,
            filename,
        } => {
            println!("[cli upload]");

            let ssh_pool = load_ssh_pool(&client, config, datadir, verbose, stream).await;

            let results = ssh_pool.upload(&filename).await;
            for r in results.iter() {
                println!("{}", r.as_ref().unwrap());
            }
        }

        Commands::Download {
            config,
            datadir,
            verbose,
            stream,
            filename,
        } => {
            println!("[cli download]");

            let ssh_pool = load_ssh_pool(&client, config, datadir, verbose, stream).await;

            let results = ssh_pool.download(&filename).await;
            for r in results.iter() {
                println!("{}", r.as_ref().unwrap());
            }
        }
    }
}
