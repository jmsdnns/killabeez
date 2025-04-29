use russh_sftp::{client::SftpSession, protocol::OpenFlags};
use std::io::Result as IOResult;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::ssh::client::Client;
use crate::ssh::errors::SshError;
use crate::ssh::output::OutputHandler;
use crate::ssh::pools::SSHConnection;

pub struct SFTPConnection<'a> {
    ssh_conn: &'a SSHConnection,
    session: SftpSession,
}

impl<'a> SFTPConnection<'a> {
    /// create sftp channel from ssh connection
    pub async fn open(ssh_conn: &'a SSHConnection) -> Result<Self, SshError> {
        let channel = ssh_conn.client.handle.channel_open_session().await?;
        channel.request_subsystem(true, "sftp").await?;
        let session = SftpSession::new(channel.into_stream()).await?;
        Ok(SFTPConnection { ssh_conn, session })
    }

    /// puts file on remote host
    pub async fn upload(
        &self,
        source: impl AsRef<Path>,
        destination: &str,
    ) -> Result<u64, SshError> {
        let mut local_file = tokio::fs::File::open(source).await?;
        let mut remote_file = self
            .session
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

    /// pulls file from remote host
    pub async fn download(
        &self,
        source: &str,
        destination: impl AsRef<Path>,
    ) -> Result<u64, SshError> {
        let mut local_file = tokio::fs::File::create(destination).await?;
        let mut remote_file = self
            .session
            .open_with_flags(source, OpenFlags::READ)
            .await?;

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
