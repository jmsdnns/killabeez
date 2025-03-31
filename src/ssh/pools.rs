use futures::{StreamExt, stream};
use std::path::PathBuf;
use std::sync::Arc;

use crate::aws::ec2::Bee;
use crate::aws::scenarios::Swarm;
use crate::config::SwarmConfig;
use crate::ssh::client::{Auth, Client};
use crate::ssh::errors::SshError;
use crate::ssh::logger::ConnectionLogger;

pub struct SSHConnection {
    client: Client,
    host: String,
    username: String,
    logger: Option<Arc<ConnectionLogger>>,
}

impl SSHConnection {
    pub async fn open(host: &str, username: &str, auth: Auth, log_dir: Option<&PathBuf>) -> Self {
        let dst = match host.split(":").collect::<Vec<&str>>()[..] {
            [h, p] => (h, p.parse::<u16>().unwrap()),
            [h] => (h, 22),
            _ => panic!("Host value makes no sense: {}", host),
        };

        // prepare  logger
        let host_id = host.replace(":", "_").replace(".", "_");
        let logger = match log_dir {
            Some(dir) => match ConnectionLogger::new(&host_id, dir) {
                Ok(logger) => Some(Arc::new(logger)),
                Err(e) => {
                    eprintln!("Failed to create logger for {}: {}", host, e);
                    None
                }
            },
            None => None,
        };

        let conn = Client::connect(dst, username, auth, logger.clone()).await;

        SSHConnection {
            client: conn.unwrap(),
            host: String::from(host),
            username: String::from(username),
            logger,
        }
    }
}

pub struct SSHPool {
    conns: Vec<SSHConnection>,
    log_dir: Option<PathBuf>,
}

impl SSHPool {
    pub fn load_key(sc: &SwarmConfig) -> Option<Auth> {
        sc.private_key_file()
            .map(|pkf| Auth::KeyFile(std::path::PathBuf::from(&pkf), None))
    }

    pub async fn new(hosts: &Vec<String>, username: &str, auth: Auth) -> SSHPool {
        Self::new_with_logging(hosts, username, auth, None).await
    }

    pub async fn new_with_logging(
        hosts: &Vec<String>,
        username: &str,
        auth: Auth,
        log_dir: Option<PathBuf>,
    ) -> SSHPool {
        let concurrency: usize = 10;

        let results = stream::iter(hosts)
            .map(|host| SSHConnection::open(host, username, auth.clone(), log_dir.as_ref()))
            .buffer_unordered(concurrency)
            .collect::<Vec<SSHConnection>>()
            .await;

        SSHPool {
            conns: results,
            log_dir,
        }
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
