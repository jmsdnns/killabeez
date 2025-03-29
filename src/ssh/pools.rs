use async_ssh2_tokio::{
    Error,
    client::{AuthMethod, Client, CommandExecutedResult, ServerCheckMethod},
};
use futures::{StreamExt, stream};

use crate::aws::ec2::Bee;
use crate::aws::scenarios::Swarm;
use crate::config::SwarmConfig;

pub struct SSHConnection {
    client: Client,
    host: String,
    username: String,
}

impl SSHConnection {
    pub async fn open(host: &str, username: &str, auth: &AuthMethod) -> Self {
        let dst = match host.split(":").collect::<Vec<&str>>()[..] {
            [h, p] => (h, p.parse::<u16>().unwrap()),
            [h] => (h, 22),
            _ => panic!("Host value makes no sense: {}", host),
        };

        let conn = Client::connect(dst, username, auth.clone(), ServerCheckMethod::NoCheck).await;

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
    pub fn load_key(sc: &SwarmConfig) -> Option<AuthMethod> {
        sc.private_key_file()
            .map(|pkf| AuthMethod::with_key_file(std::path::Path::new(&pkf), None))
    }

    pub async fn new(hosts: &Vec<String>, username: &str, auth: &AuthMethod) -> SSHPool {
        let concurrency: usize = 10;

        let results = stream::iter(hosts)
            .map(|host| SSHConnection::open(host, username, auth))
            .buffer_unordered(concurrency)
            .collect::<Vec<SSHConnection>>()
            .await;

        SSHPool { conns: results }
    }

    pub async fn exec(&self, cmd: &str) -> Vec<CommandExecutedResult> {
        stream::iter(self.conns.iter())
            .map(|c| c.client.execute(cmd))
            .buffer_unordered(10)
            .collect::<Vec<Result<CommandExecutedResult, Error>>>()
            .await
            .iter()
            .map(|o| o.as_ref().unwrap().to_owned())
            .collect::<Vec<CommandExecutedResult>>()
    }
}

pub fn print_results(results: Vec<CommandExecutedResult>) {
    for r in results.iter() {
        print!("{}", r.stdout);
    }
}
