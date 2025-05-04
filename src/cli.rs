use clap::{Parser, Subcommand};

use crate::actions::commands;
use crate::ssh::files::DEFAULT_LOCAL_ROOT;

const ABOUT_CLI: &str = "killabeez: a CLI for creating traffic jams of arbitrary scale";
const DEFAULT_CONFIG: &str = "swarm.toml";
const DEFAULT_PLAN: &str = "swarm.plan";

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

    /// Run an execution plan
    Plan {
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

        /// Path to the planfile
        #[arg(short, long, default_value_t = DEFAULT_PLAN.to_string())]
        planfile: String,
    },
}

pub async fn run() {
    let args = Cli::parse();

    match args.command {
        Commands::Init { config } => {
            println!("[cli init]");
            commands::cmd_init(&config).await;
        }

        Commands::Tagged { config } => {
            println!("[cli tagged]");
            commands::cmd_tagged(&config).await;
        }

        Commands::Terminate { config } => {
            println!("[cli terminate]");
            commands::cmd_terminate(&config).await;
        }

        Commands::Exec {
            config,
            datadir,
            verbose,
            remote,
            command,
        } => {
            println!("[cli exec]");
            commands::cmd_execute(&config, &command, &datadir, verbose, remote).await;
        }

        Commands::Upload {
            config,
            datadir,
            verbose,
            remote,
            source,
        } => {
            println!("[cli upload]");
            commands::cmd_upload(&config, &source, &datadir, verbose, remote).await;
        }

        Commands::Download {
            config,
            datadir,
            verbose,
            remote,
            source,
        } => {
            println!("[cli download]");
            commands::cmd_download(&config, &source, &datadir, verbose, remote).await;
        }

        Commands::Plan {
            config,
            datadir,
            verbose,
            remote,
            planfile,
        } => {
            println!("[cli plan]");
            commands::cmd_plan(&config, &planfile, &datadir, verbose, remote).await;
        }
    }
}
