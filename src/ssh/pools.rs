use futures::{StreamExt, TryFutureExt, future, stream};
use std::path::PathBuf;
use std::sync::Arc;

use crate::aws::ec2::Bee;
use crate::aws::scenarios::Swarm;
use crate::config::SwarmConfig;
use crate::ssh::client::{Auth, Client, Output};
use crate::ssh::errors::SshError;
use crate::ssh::files::SFTPConnection;
use crate::ssh::io::{IOHandler, RemoteIO, StreamIO};

/// tracks the basic elements of an SSH connection
pub struct SSHConnection {
    /// killabeez ssh client
    pub client: Client,

    /// remote host address, as hostname or IP
    pub host: String,

    /// remote username
    pub username: String,
}

impl SSHConnection {
    /// Opens a connection to `host` and prepares the output handler for the
    /// ssh client
    pub async fn open(host: &str, username: &str, auth: Auth, output: Output) -> Self {
        let dst = match host.split(":").collect::<Vec<&str>>()[..] {
            [h, p] => (h, p.parse::<u16>().unwrap()),
            [h] => (h, 22),
            _ => panic!("Host value makes no sense: {}", host),
        };

        let host_id = host.replace(":", "_").replace(".", "_");

        let output_handler: Arc<dyn IOHandler> = match output {
            Output::Stream(log_root, verbose) => {
                match StreamIO::new(&host_id, &log_root, verbose) {
                    Ok(logger) => Arc::new(logger) as Arc<dyn IOHandler>,
                    Err(e) => panic!("ERROR boo {}", e),
                }
            }
            Output::Remote(log_root, verbose) => {
                match RemoteIO::new(&host_id, Some(&log_root.clone()), verbose) {
                    Ok(logger) => Arc::new(logger) as Arc<dyn IOHandler>,
                    Err(e) => panic!("ERROR boo {}", e),
                }
            }
        };

        let conn = Client::connect(dst, username, auth, output_handler.clone()).await;

        SSHConnection {
            client: conn.unwrap(),
            host: String::from(host),
            username: String::from(username),
        }
    }
}

pub struct SSHPool {
    conns: Vec<SSHConnection>,
    output: Output,
}

impl SSHPool {
    pub fn load_key(sc: &SwarmConfig) -> Option<Auth> {
        sc.private_key_file()
            .map(|pkf| Auth::KeyFile(std::path::PathBuf::from(&pkf), None))
    }

    pub async fn new(hosts: &Vec<String>, username: &str, auth: Auth, output: Output) -> SSHPool {
        let concurrency: usize = 10;
        let results = stream::iter(hosts)
            .map(|host| SSHConnection::open(host, username, auth.clone(), output.clone()))
            .buffer_unordered(concurrency)
            .collect::<Vec<SSHConnection>>()
            .await;

        SSHPool {
            conns: results,
            output,
        }
    }

    pub async fn execute(&self, command: &str) -> Vec<Result<u32, SshError>> {
        stream::iter(self.conns.iter())
            .map(|c| c.client.execute(command))
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
