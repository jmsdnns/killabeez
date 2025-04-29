use futures::{StreamExt, TryFutureExt, future, stream};
use std::path::PathBuf;
use std::sync::Arc;

use crate::aws::ec2::Bee;
use crate::aws::scenarios::Swarm;
use crate::config::SwarmConfig;
use crate::ssh::client::{Auth, Client};
use crate::ssh::errors::SshError;
use crate::ssh::files::SFTPConnection;
use crate::ssh::io::{IOConfig, IOHandler, RemoteIO, StreamIO};

use super::files::SessionData;

/// tracks the basic elements of an SSH connection
pub struct SSHConnection {
    /// killabeez ssh client
    pub client: Client,

    /// thread safe IO handler
    pub io_handler: Arc<dyn IOHandler>,

    /// settings for data management
    pub data: SessionData,

    /// remote host address, as hostname or IP
    pub host: String,

    /// remote username
    pub username: String,
}

impl SSHConnection {
    /// Opens a connection to `host` and prepares the io handler for the ssh client
    pub async fn open(host: &str, username: &str, auth: Auth, io_config: IOConfig) -> Self {
        let dst = match host.split(":").collect::<Vec<&str>>()[..] {
            [h, p] => (h, p.parse::<u16>().unwrap()),
            [h] => (h, 22),
            _ => panic!("Host value makes no sense: {}", host),
        };

        let (data, io_handler): (SessionData, Arc<dyn IOHandler>) = match io_config {
            IOConfig::Stream(local_root, verbose) => {
                let data = SessionData::new(host.to_string(), local_root.clone(), None).unwrap();
                match StreamIO::new(&data.local_root, verbose) {
                    Ok(logger) => (data, Arc::new(logger) as Arc<dyn IOHandler>),
                    Err(e) => panic!("ERROR boo {}", e),
                }
            }
            IOConfig::Remote(local_root, remote_root, verbose) => {
                let data =
                    SessionData::new(host.to_string(), local_root.clone(), remote_root.clone())
                        .unwrap();
                match RemoteIO::new(&data.remote_root, verbose) {
                    Ok(logger) => (data, Arc::new(logger) as Arc<dyn IOHandler>),
                    Err(e) => panic!("ERROR boo {}", e),
                }
            }
        };

        let conn = Client::connect(dst, username, auth).await;

        SSHConnection {
            client: conn.unwrap(),
            io_handler,
            data,
            host: String::from(host),
            username: String::from(username),
        }
    }

    pub async fn execute(&self, command: &str) -> Result<u32, SshError> {
        let command = self.io_handler.update_command(command);
        self.client.execute(&command, &self.io_handler).await
    }

    pub async fn finish(&self) -> Result<(), SshError> {
        let sftp = SFTPConnection::open(self).await?;

        let artifacts = self.io_handler.artifacts();
        for a in self.io_handler.artifacts() {
            let local_path = self.data.local_root.clone().join(&a);
            let remote_path = self.data.remote_root.clone().join(&a);
            sftp.download(&remote_path.display().to_string(), local_path)
                .await?;
        }

        Ok(())
    }
}

pub struct SSHPool {
    conns: Vec<SSHConnection>,
    io_config: IOConfig,
}

impl SSHPool {
    pub fn load_key(sc: &SwarmConfig) -> Option<Auth> {
        sc.private_key_file()
            .map(|pkf| Auth::KeyFile(std::path::PathBuf::from(&pkf), None))
    }

    pub async fn new(
        hosts: &Vec<String>,
        username: &str,
        auth: Auth,
        io_config: IOConfig,
    ) -> SSHPool {
        let concurrency: usize = 10;
        let results = stream::iter(hosts)
            .map(|host| SSHConnection::open(host, username, auth.clone(), io_config.clone()))
            .buffer_unordered(concurrency)
            .collect::<Vec<SSHConnection>>()
            .await;

        SSHPool {
            conns: results,
            io_config,
        }
    }

    pub async fn finish(&self) -> Vec<Result<(), SshError>> {
        stream::iter(self.conns.iter())
            .map(|c| c.finish())
            .buffer_unordered(10)
            .collect::<Vec<Result<(), SshError>>>()
            .await
    }

    pub async fn execute(&self, command: &str) -> Vec<Result<u32, SshError>> {
        stream::iter(self.conns.iter())
            .map(|c| c.execute(command))
            .buffer_unordered(10)
            .collect::<Vec<Result<u32, SshError>>>()
            .await
    }

    pub async fn upload(&self, filename: &str) -> Vec<Result<u64, SshError>> {
        stream::iter(self.conns.iter())
            .map(|c| {
                Box::pin(async {
                    SFTPConnection::open(c)
                        .await
                        .unwrap()
                        .upload(filename, filename)
                        .await
                })
            })
            .buffer_unordered(10)
            .collect::<Vec<Result<u64, SshError>>>()
            .await
    }

    pub async fn download(&self, filename: &str) -> Vec<Result<u64, SshError>> {
        stream::iter(self.conns.iter())
            .map(|c| {
                Box::pin(async {
                    SFTPConnection::open(c)
                        .await
                        .unwrap()
                        .download(filename, filename)
                        .await
                })
            })
            .buffer_unordered(10)
            .collect::<Vec<Result<u64, SshError>>>()
            .await
    }
}
