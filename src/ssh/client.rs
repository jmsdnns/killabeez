use russh::client::{Config, Handle, Handler};
use russh::keys::PrivateKey;
use russh_sftp::{client::SftpSession, protocol::OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::ssh::errors::SshError;
use crate::ssh::output::OutputHandler;

#[derive(Debug, Clone)]
pub enum Auth {
    Password(String),
    KeyFile(PathBuf, Option<String>),
    KeyData(String, Option<String>),
}

pub struct ClientHandler;

impl Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(&mut self, _: &russh::keys::PublicKey) -> Result<bool, Self::Error> {
        Ok(true) // TODO: should check keys are valid, etc
    }
}

pub struct Client {
    handle: Handle<ClientHandler>,
    logger: Option<Arc<dyn OutputHandler>>,
}

impl Client {
    pub async fn connect(
        address: impl std::net::ToSocketAddrs,
        username: &str,
        auth: Auth,
        logger: Option<Arc<dyn OutputHandler>>,
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

        Ok(Self { handle, logger })
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

    pub async fn execute<F, G>(
        &self,
        command: &str,
        mut stdout_handler: F,
        mut stderr_handler: G,
    ) -> Result<u32, SshError>
    where
        F: FnMut(&[u8]) + Send,
        G: FnMut(&[u8]) + Send,
    {
        let mut channel = self.handle.channel_open_session().await?;
        channel.exec(true, command).await?;

        let mut exit_status = None;

        while let Some(msg) = channel.wait().await {
            match msg {
                russh::ChannelMsg::Data { ref data } => {
                    stdout_handler(data);
                }
                russh::ChannelMsg::ExtendedData { ref data, ext: 1 } => {
                    stderr_handler(data);
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

    pub async fn execute_and_print(&self, command: &str) -> Result<u32, SshError> {
        use std::io::Write;

        let mut stdout_handler = |data: &[u8]| {
            std::io::stdout().write_all(data).unwrap();
            std::io::stdout().flush().unwrap();
            if let Some(logger) = &self.logger {
                if let Err(e) = logger.stdout(data) {
                    eprintln!("Failed to log stdout: {}", e);
                }
            }
        };

        let mut stderr_handler = |data: &[u8]| {
            std::io::stderr().write_all(data).unwrap();
            std::io::stderr().flush().unwrap();
            if let Some(logger) = &self.logger {
                if let Err(e) = logger.stderr(data) {
                    eprintln!("Failed to log stderr: {}", e);
                }
            }
        };

        self.execute(command, &mut stdout_handler, &mut stderr_handler)
            .await
    }

    async fn sftp_session(&self) -> Result<SftpSession, SshError> {
        let channel = self.handle.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let session = SftpSession::new(channel.into_stream()).await?;
        Ok(session)
    }

    pub async fn upload(
        &self,
        source: impl AsRef<Path>,
        destination: &str,
    ) -> Result<u64, SshError> {
        let session = self.sftp_session().await?;

        let mut local_file = tokio::fs::File::open(source).await?;
        let mut remote_file = session
            .open_with_flags(
                destination,
                OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
            )
            .await?;

        // stream 32k chunks
        let mut buffer = [0u8; 32768];
        let mut total_bytes = 0;
        loop {
            match local_file.read(&mut buffer).await? {
                0 => break,
                bytes_read => {
                    remote_file.write_all(&buffer[..bytes_read]).await?;
                    total_bytes += bytes_read as u64;
                }
            }
        }

        remote_file.flush().await?;
        remote_file.shutdown().await?;
        Ok(total_bytes)
    }

    pub async fn download(
        &self,
        source: &str,
        destination: impl AsRef<Path>,
    ) -> Result<u64, SshError> {
        let session = self.sftp_session().await?;

        let mut remote_file = session.open_with_flags(source, OpenFlags::READ).await?;
        let mut local_file = tokio::fs::File::create(destination).await?;

        // stream 32k chunks
        let mut buffer = [0u8; 32768];
        let mut total_bytes = 0;
        loop {
            match remote_file.read(&mut buffer).await? {
                0 => break,
                bytes_read => {
                    local_file.write_all(&buffer[..bytes_read]).await?;
                    total_bytes += bytes_read as u64;
                }
            }
        }

        local_file.flush().await?;
        Ok(total_bytes)
    }
}
