use clap::{Parser, Subcommand};

use crate::beez;
use crate::config::AppConfig;

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

pub async fn run(ac: AppConfig) {
    let args = Cli::parse();

    let Ok(client) = beez::mk_client(&ac).await else {
        panic!("[client] Waaaah");
    };

    match args.command {
        Commands::Init { name, count } => {
            println!("[init] {name}");
            println!("[init] {:?}", count);
        }
        Commands::Terminate { name } => {
            println!("[terminate] {name}");
        }
        Commands::Exec { name, script } => {
            println!("[exec] {name} {:?}", script);
        }
    }
}
