use std::fs;
use std::path::PathBuf;

use aws_sdk_ec2::Client;

use crate::actions::plans::{ParsedAction, parse_commands};
use crate::aws::ec2::mk_client as mk_ec2;
use crate::aws::scenarios::{AWSNetwork, Swarm};
use crate::aws::tagged;
use crate::aws::tagged::mk_client as mk_tagged;
use crate::config::SwarmConfig;
use crate::ssh::io::IOConfig;
use crate::ssh::pools::SSHPool;

pub async fn load_ssh_pool(
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

pub async fn cmd_init(config: &str) {
    let sc = SwarmConfig::read(config).unwrap();
    println!("{}", sc);

    let client = mk_ec2().await;
    let network = AWSNetwork::init(&client, &sc).await.unwrap();
    let swarm = Swarm::init(&client, &sc, &network).await.unwrap();
    println!("{}", swarm);
}

pub async fn cmd_tagged(config: &str) {
    let sc = SwarmConfig::read(config).unwrap();
    println!("{}", sc);

    let client = mk_tagged().await;
    let arns = tagged::list_all_tagged(&client, &sc).await.unwrap();

    match arns.len() {
        0 => println!("[list_all_tagged] 0 resources found"),
        count => {
            println!("[list_all_tagged] {} resources found", count);
            for arn in arns.clone() {
                println!("{}", arn);
            }
        }
    }
}

pub async fn cmd_terminate(config: &str) {
    let sc = SwarmConfig::read(config).unwrap();
    println!("{}", sc);

    let client = mk_ec2().await;
    Swarm::drop(&client, &sc).await;
    AWSNetwork::drop(&client, &sc).await;
}

pub async fn cmd_execute(config: &str, command: &str, datadir: &str, verbose: bool, remote: bool) {
    let client = mk_ec2().await;
    let ssh_pool = load_ssh_pool(
        &client,
        config.to_string(),
        datadir.to_string(),
        verbose,
        remote,
    )
    .await;

    ssh_pool.execute(command).await;
    println!("[cmd_execute] fetching remote artifacts");
    ssh_pool.finish().await;
}

pub async fn cmd_upload(config: &str, source: &str, datadir: &str, verbose: bool, remote: bool) {
    let client = mk_ec2().await;
    let ssh_pool = load_ssh_pool(
        &client,
        config.to_string(),
        datadir.to_string(),
        verbose,
        remote,
    )
    .await;

    let results = ssh_pool.upload(source).await;
    for r in results.iter() {
        println!("{}", r.as_ref().unwrap());
    }
}

pub async fn cmd_download(config: &str, source: &str, datadir: &str, verbose: bool, remote: bool) {
    let client = mk_ec2().await;
    let ssh_pool = load_ssh_pool(
        &client,
        config.to_string(),
        datadir.to_string(),
        verbose,
        remote,
    )
    .await;

    let results = ssh_pool.download(source).await;
    for r in results.iter() {
        println!("{}", r.as_ref().unwrap());
    }
}

pub async fn cmd_plan(config: &str, planfile: &str, datadir: &str, verbose: bool, remote: bool) {
    let client = mk_ec2().await;
    let ssh_pool = load_ssh_pool(
        &client,
        config.to_string(),
        datadir.to_string(),
        verbose,
        remote,
    )
    .await;

    let content = fs::read_to_string("commands.plan").unwrap();

    let mut input = content.as_str();
    match parse_commands(&mut input) {
        Ok(commands) => {
            println!("Successfully parsed {} commands:", commands.len());

            let tasks: Vec<_> = commands
                .iter()
                .enumerate()
                .map(async |(i, cmd)| match cmd {
                    ParsedAction::Execute { command } => {
                        ssh_pool.execute(command.as_str()).await;
                    }
                    ParsedAction::Upload { source } => {
                        ssh_pool.upload(source.as_str()).await;
                    }
                    ParsedAction::Download { source } => {
                        ssh_pool.download(source.as_str()).await;
                    }
                })
                .collect();
            use futures::future::join_all;
            join_all(tasks).await;
        }
        Err(e) => {
            println!("Error parsing commands: {}", e);
        }
    }

    println!("[cmd_execute] fetching remote artifacts");
    ssh_pool.finish().await;
}
