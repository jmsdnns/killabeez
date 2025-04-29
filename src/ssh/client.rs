use russh::client::{Config, Handle, Handler};
use russh::keys::PrivateKey;
use russh_sftp::{client::SftpSession, protocol::OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::ssh::errors::SshError;
use crate::ssh::output::OutputHandler;

/// The options available for SSH authentication
#[derive(Debug, Clone)]
pub enum Auth {
    Password(String),
    KeyFile(PathBuf, Option<String>),
    KeyData(String, Option<String>),
}

/// The choices for output handling
#[derive(Debug, Clone)]
pub enum Output {
    Stream(PathBuf, bool),
    Remote(PathBuf, bool),
}

pub struct ClientHandler;

impl Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(&mut self, _: &russh::keys::PublicKey) -> Result<bool, Self::Error> {
        Ok(true) // TODO: should check keys are valid, etc
    }
}

/// Associates the russh client without output handling
pub struct Client {
    pub handle: Handle<ClientHandler>,
    output: Arc<dyn OutputHandler>,
}

impl Client {
    pub async fn connect(
        address: impl std::net::ToSocketAddrs,
        username: &str,
        auth: Auth,
        output: Arc<dyn OutputHandler>,
    ) -> Result<Self, SshError> {
        let config = Arc::new(Config::default());
        let addr = address
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| SshError::AddressError("Invalid address".to_string()))?;

        let mut handle = russh::client::connect(config, addr, ClientHandler).await?;

        match auth {
            Auth::Password(password) => {
                let auth_res = handle.authenticate_password(username, password).await?;
                if !auth_res.success() {
                    return Err(SshError::AuthenticationFailed(
                        "Password authentication failed".to_string(),
                    ));
                }
            }
            Auth::KeyFile(path, passphrase) => {
                let key = russh::keys::load_secret_key(path, passphrase.as_deref())?;
                Client::auth_with_key(key, username, &mut handle).await?
            }
            Auth::KeyData(key_data, passphrase) => {
                let key = russh::keys::decode_secret_key(&key_data, passphrase.as_deref())?;
                Client::auth_with_key(key, username, &mut handle).await?
            }
        }

        Ok(Self { handle, output })
    }

    async fn auth_with_key(
        key: PrivateKey,
        username: &str,
        handle: &mut Handle<ClientHandler>,
    ) -> Result<(), SshError> {
        let key = Arc::new(key);
        let hash = handle.best_supported_rsa_hash().await?.ok_or_else(|| {
            SshError::AuthenticationFailed("No suitable RSA hash algorithm found".to_string())
        })?;

        let auth_res = handle
            .authenticate_publickey(username, russh::keys::PrivateKeyWithHashAlg::new(key, hash))
            .await?;

        match auth_res.success() {
            true => Ok(()),
            false => Err(SshError::AuthenticationFailed(
                "Key authentication failed".to_string(),
            )),
        }
    }

    pub async fn disconnect(&self) -> Result<(), SshError> {
        self.handle
            .disconnect(russh::Disconnect::ByApplication, "", "")
            .await?;
        Ok(())
    }

    /// run command on remote host
    pub async fn execute(&self, command: &str) -> Result<u32, SshError> {
        // output choice may need to modify command string
        let command = self.output.as_ref().update_command(command);

        // run command
        let mut channel = self.handle.channel_open_session().await?;
        channel.exec(true, command).await?;

        // handle stdout & stderr from remote until exit code
        let mut exit_status = None;
        while let Some(msg) = channel.wait().await {
            match msg {
                russh::ChannelMsg::Data { ref data } => {
                    self.output.as_ref().stdout(data);
                }
                russh::ChannelMsg::ExtendedData { ref data, ext: 1 } => {
                    self.output.as_ref().stderr(data);
                }
                russh::ChannelMsg::ExitStatus { exit_status: code } => {
                    exit_status = Some(code);
                }
                _ => {}
            }
        }

        exit_status
            .ok_or_else(|| SshError::CommandError("Command didn't exit properly".to_string()))
    }
}
