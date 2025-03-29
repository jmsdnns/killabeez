use std::env::VarError;
use std::fmt;
use std::net::AddrParseError;

#[derive(Debug)]
pub enum SshError {
    ConnectionError(russh::Error),
    AuthenticationFailed(String),
    AddressError(String),
    KeyError(russh::keys::Error),
    IoError(std::io::Error),
    SftpError(russh_sftp::protocol::StatusCode),
    SessionError(String),
    CommandError(String),
    ChannelError(String),
    SubsystemError(String),
    InvalidResponse(String),
    EnvError(VarError),
    AddrParseError(AddrParseError),
    UnsupportedOperation(String),
    TimeoutError(String),
}

impl fmt::Display for SshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SshError::ConnectionError(e) => write!(f, "SSH connection error: {}", e),
            SshError::AuthenticationFailed(msg) => write!(f, "Authentication failed: {}", msg),
            SshError::AddressError(msg) => write!(f, "Address resolution failed: {}", msg),
            SshError::KeyError(e) => write!(f, "Key loading failed: {}", e),
            SshError::IoError(e) => write!(f, "IO error: {}", e),
            SshError::SftpError(e) => write!(f, "SFTP error: {}", e),
            SshError::SessionError(msg) => write!(f, "Session error: {}", msg),
            SshError::CommandError(msg) => write!(f, "Command execution error: {}", msg),
            SshError::ChannelError(msg) => write!(f, "SSH channel error: {}", msg),
            SshError::SubsystemError(msg) => write!(f, "Subsystem error: {}", msg),
            SshError::InvalidResponse(msg) => write!(f, "Invalid server response: {}", msg),
            SshError::EnvError(e) => write!(f, "Environment variable error: {}", e),
            SshError::AddrParseError(e) => write!(f, "Address parsing error: {}", e),
            SshError::UnsupportedOperation(msg) => write!(f, "Unsupported operation: {}", msg),
            SshError::TimeoutError(msg) => write!(f, "Operation timed out: {}", msg),
        }
    }
}

impl std::error::Error for SshError {}

impl From<std::io::Error> for SshError {
    fn from(err: std::io::Error) -> Self {
        SshError::IoError(err)
    }
}

impl From<russh::Error> for SshError {
    fn from(err: russh::Error) -> Self {
        SshError::ConnectionError(err)
    }
}

impl From<russh::keys::Error> for SshError {
    fn from(err: russh::keys::Error) -> Self {
        SshError::KeyError(err)
    }
}

impl From<russh_sftp::protocol::StatusCode> for SshError {
    fn from(err: russh_sftp::protocol::StatusCode) -> Self {
        SshError::SftpError(err)
    }
}

impl From<VarError> for SshError {
    fn from(err: VarError) -> Self {
        SshError::EnvError(err)
    }
}

impl From<AddrParseError> for SshError {
    fn from(err: AddrParseError) -> Self {
        SshError::AddrParseError(err)
    }
}

impl From<russh_sftp::client::error::Error> for SshError {
    fn from(err: russh_sftp::client::error::Error) -> Self {
        SshError::SessionError(format!("SFTP error: {}", err))
    }
}

impl From<String> for SshError {
    fn from(err: String) -> Self {
        SshError::InvalidResponse(err)
    }
}

impl From<&str> for SshError {
    fn from(err: &str) -> Self {
        SshError::InvalidResponse(err.to_string())
    }
}
