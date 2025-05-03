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
    /// Prepares resources for bringing swarm online
    Init {
        /// Path to swarm config
        #[arg(short, long, value_name = "SWARM CONFIG", default_value_t = DEFAULT_CONFIG.to_string())]
        config: String,
    },

    /// Lists all resources tagged by swarm config
    Tagged {
        /// Path to swarm config
        #[arg(short, long, value_name = "SWARM CONFIG", default_value_t = DEFAULT_CONFIG.to_string())]
        config: String,
    },

    /// Terminate all managed resources
    Terminate {
        /// Path to swarm config
        #[arg(short, long, value_name = "SWARM CONFIG", default_value_t = DEFAULT_CONFIG.to_string())]
        config: String,
    },

    /// Execute command on swarm
    Exec {
        /// Path to swarm config
        #[arg(short, long, value_name = "SWARM CONFIG", default_value_t = DEFAULT_CONFIG.to_string())]
        config: String,

        /// Directory path for storing swarm output
        #[arg(short, long, default_value_t = DEFAULT_LOCAL_ROOT.to_string())]
        datadir: String,

        /// Also write stdout/stderr to console
        #[arg(short, long, default_value_t = false)]
        verbose: bool,

        /// Disable output streaming and write output to remote files until session end
        #[arg(short, long, default_value_t = false)]
        remote: bool,

        /// A string containing the command to execute
        #[arg(required = true)]
        command: String,
    },

    /// Upload a file to swarm
    Upload {
        /// Path to swarm config
        #[arg(short, long, value_name = "SWARM CONFIG", default_value_t = DEFAULT_CONFIG.to_string())]
        config: String,

        /// Directory path for storing swarm output
        #[arg(short, long, value_name = "KB DATA", default_value_t = DEFAULT_LOCAL_ROOT.to_string())]
        datadir: String,

        /// Also write stdout/stderr to console
        #[arg(short, long, default_value_t = false)]
        verbose: bool,

        /// Disable output streaming and write output to remote files until session end
        #[arg(short, long, default_value_t = false)]
        remote: bool,

        /// the local file to be uploaded
        #[arg(required = true, value_name = "FILE")]
        source: String,
    },

    /// Download a file from swarm
    Download {
        /// Path to swarm config
        #[arg(short, long, value_name = "SWARM CONFIG", default_value_t = DEFAULT_CONFIG.to_string())]
        config: String,

        /// Directory path for storing swarm output
        #[arg(short, long, default_value_t = DEFAULT_LOCAL_ROOT.to_string())]
        datadir: String,

        /// Also write stdout/stderr to console
        #[arg(short, long, default_value_t = false)]
        verbose: bool,

        /// Disable output streaming and write output to remote files until session end
        #[arg(short, long, default_value_t = false)]
        remote: bool,

        /// the remote file to be downloaded
        #[arg(required = true, value_name = "FILE")]
        source: String,
    },
}

async fn load_ssh_pool(
    client: &Client,
    config: String,
    datadir: String,
    verbose: bool,
    remote: bool,
) -> SSHPool {
    let sc = SwarmConfig::read(&config).unwrap();
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

    let io_config = match remote {
        true => IOConfig::Remote(PathBuf::from(datadir.clone()), None, verbose),
        false => IOConfig::Stream(PathBuf::from(datadir.clone()), verbose),
    };

    SSHPool::new(&hosts, &sc.username.unwrap(), auth, io_config).await
}

pub async fn run() {
    let args = Cli::parse();

    let client = ec2::mk_client().await;

    match args.command {
        Commands::Init { config } => {
            println!("[cli init]");
            let sc = SwarmConfig::read(&config).unwrap();
            println!("{}", sc);

            let network = AWSNetwork::init(&client, &sc).await.unwrap();
            let swarm = Swarm::init(&client, &sc, &network).await.unwrap();
            println!("{}", swarm);
        }

        Commands::Tagged { config } => {
            println!("[cli tagged]");
            let sc = SwarmConfig::read(&config).unwrap();

            tagged::list_all_tagged(&sc).await;
        }

        Commands::Terminate { config } => {
            println!("[cli terminate]");
            let sc = SwarmConfig::read(&config).unwrap();
            println!("{}", sc);

            Swarm::drop(&client, &sc).await;
            AWSNetwork::drop(&client, &sc).await;
        }

        Commands::Exec {
            config,
            datadir,
            verbose,
            remote,
            command,
        } => {
            println!("[cli exec]");

            let ssh_pool = load_ssh_pool(&client, config, datadir, verbose, remote).await;
            ssh_pool.execute(&command).await;

            println!("[cli exec] fetching remote artifacts");
            ssh_pool.finish().await;
        }

        Commands::Upload {
            config,
            datadir,
            verbose,
            remote,
            source,
        } => {
            println!("[cli upload]");

            let ssh_pool = load_ssh_pool(&client, config, datadir, verbose, remote).await;

            let results = ssh_pool.upload(&source).await;
            for r in results.iter() {
                println!("{}", r.as_ref().unwrap());
            }
        }

        Commands::Download {
            config,
            datadir,
            verbose,
            remote,
            source,
        } => {
            println!("[cli download]");

            let ssh_pool = load_ssh_pool(&client, config, datadir, verbose, remote).await;

            let results = ssh_pool.download(&source).await;
            for r in results.iter() {
                println!("{}", r.as_ref().unwrap());
            }
        }
    }
}
