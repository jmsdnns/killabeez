// use async_ssh2_tokio::{
//     Error,
//     client::{Auth, Client, CommandExecutedResult, ServerCheckMethod},
// };
use futures::{StreamExt, stream};

use crate::aws::ec2::Bee;
use crate::aws::scenarios::Swarm;
use crate::config::SwarmConfig;
use crate::ssh::client::{Auth, Client};
use crate::ssh::errors::SshError;

pub struct SSHConnection {
    client: Client,
    host: String,
    username: String,
}

impl SSHConnection {
    pub async fn open(host: &str, username: &str, auth: Auth) -> Self {
        let dst = match host.split(":").collect::<Vec<&str>>()[..] {
            [h, p] => (h, p.parse::<u16>().unwrap()),
            [h] => (h, 22),
            _ => panic!("Host value makes no sense: {}", host),
        };

        let conn = Client::connect(dst, username, auth).await;

        SSHConnection {
            client: conn.unwrap(),
            host: String::from(host),
            username: String::from(username),
        }
    }
}

pub struct SSHPool {
    conns: Vec<SSHConnection>,
}

impl SSHPool {
    pub fn load_key(sc: &SwarmConfig) -> Option<Auth> {
        sc.private_key_file()
            .map(|pkf| Auth::KeyFile(std::path::PathBuf::from(&pkf), None))
    }

    pub async fn new(hosts: &Vec<String>, username: &str, auth: Auth) -> SSHPool {
        let concurrency: usize = 10;

        let results = stream::iter(hosts)
            .map(|host| SSHConnection::open(host, username, auth.clone()))
            .buffer_unordered(concurrency)
            .collect::<Vec<SSHConnection>>()
            .await;

        SSHPool { conns: results }
    }

    pub async fn execute(&self, command: &str) -> Vec<Result<u32, SshError>> {
        stream::iter(self.conns.iter())
            .map(|c| c.client.execute_and_print(command))
            .buffer_unordered(10)
            .collect::<Vec<Result<u32, SshError>>>()
            .await
    }

    pub async fn upload(&self, filename: &str) -> Vec<Result<u64, SshError>> {
        stream::iter(self.conns.iter())
            .map(|c| c.client.upload(filename, filename))
            .buffer_unordered(10)
            .collect::<Vec<Result<u64, SshError>>>()
            .await
    }

    pub async fn download(&self, filename: &str) -> Vec<Result<u64, SshError>> {
        stream::iter(self.conns.iter())
            .map(|c| c.client.download(filename, filename))
            .buffer_unordered(10)
            .collect::<Vec<Result<u64, SshError>>>()
            .await
    }
}
