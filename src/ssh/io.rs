use std::fs::{File, OpenOptions};
use std::io::{self, Result as IOResult, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use crate::ssh::files::SessionData;

const STDOUT_FILE: &str = "stdout.log";
const STDERR_FILE: &str = "stderr.log";

/// The choices for output handling
#[derive(Debug, Clone)]
pub enum IOConfig {
    Stream(PathBuf, bool),
    Remote(PathBuf, Option<PathBuf>, bool),
}

/// A trait focused on managing stdout and stderr
pub trait IOHandler: Send + Sync {
    /// Processes streams of stdout from remote host
    fn stdout(&self, data: &[u8]) -> io::Result<()>;

    /// Processes streams of stderr from remote host
    fn stderr(&self, data: &[u8]) -> io::Result<()>;

    /// Modifies a command string before execution
    fn update_command(&self, command: &str) -> String {
        String::from(command)
    }
}

/// Write stdout & stderr to the remote filesystem, minimizing chat between
/// local machine and remotes
pub struct RemoteIO {
    session_data: SessionData,
    out_path: PathBuf,
    err_path: PathBuf,
    verbose: bool,
}

impl RemoteIO {
    pub fn new(session_data: SessionData, verbose: bool) -> io::Result<Self> {
        let out_path =
            PathBuf::from_iter([session_data.remote_root.clone(), PathBuf::from(STDOUT_FILE)]);

        let err_path =
            PathBuf::from_iter([session_data.remote_root.clone(), PathBuf::from(STDERR_FILE)]);

        Ok(Self {
            session_data,
            out_path,
            err_path,
            verbose,
        })
    }
}

impl IOHandler for RemoteIO {
    /// stdout is not sent from remote unless verbose flag is used
    fn stdout(&self, data: &[u8]) -> io::Result<()> {
        if self.verbose {
            std::io::stderr().write_all(data).unwrap();
            std::io::stderr().flush().unwrap();
        }

        Ok(())
    }

    /// stderr is not sent from remote unless verbose flag is used
    fn stderr(&self, data: &[u8]) -> io::Result<()> {
        if self.verbose {
            std::io::stderr().write_all(data).unwrap();
            std::io::stderr().flush().unwrap();
        }

        Ok(())
    }

    /// Wraps a command string with code that adds timestamps to output and redirects
    /// both stdout and stderr to files. Output is written to both files and the console
    /// if `verbose` is true
    fn update_command(&self, cmd: &str) -> String {
        let outh = format!(
            r#"awk '{{ print strftime("[%Y-%m-%d %H:%M:%S]"), $0, "" }}' >> {}"#,
            &self.out_path.display()
        );

        let errh = format!(
            r#"awk '{{ print strftime("[%Y-%m-%d %H:%M:%S]"), $0, "" }}' >> {}"#,
            &self.err_path.display()
        );

        if self.verbose {
            // NOTE: this feels hacky, but it works too
            format!("{} > >(tee >({}) >&1) 2> >(tee >({}) >&2)", cmd, outh, errh)
        } else {
            format!("{} > >({}) 2> >({})", cmd, outh, errh)
        }
    }
}

/// Threadsafe struct that handles writing output streams to local files
pub struct StreamIO {
    session_data: SessionData,
    stdout_file: Arc<Mutex<File>>,
    stderr_file: Arc<Mutex<File>>,
    verbose: bool,
}

impl StreamIO {
    pub fn new(session_data: SessionData, verbose: bool) -> io::Result<Self> {
        let stdout_path = session_data.local_root.join(STDOUT_FILE);
        let stdout_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(stdout_path)?;

        let stderr_path = session_data.local_root.join(STDERR_FILE);
        let stderr_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(stderr_path)?;

        Ok(Self {
            session_data,
            stdout_file: Arc::new(Mutex::new(stdout_file)),
            stderr_file: Arc::new(Mutex::new(stderr_file)),
            verbose,
        })
    }

    pub fn log_to_file(&self, file: &mut MutexGuard<File>, data: &[u8]) -> io::Result<()> {
        let timestamp = chrono::Local::now()
            .format("[%Y-%m-%d %H:%M:%S] ")
            .to_string();

        file.write_all(timestamp.as_bytes())?;
        file.write_all(data)?;
        if !data.ends_with(b"\n") {
            file.write_all(b"\n")?;
        }
        file.flush()
    }
}

impl IOHandler for StreamIO {
    /// writes remote stdout to local file as it is streamed. will write copy
    /// to console if `verbose` is true
    fn stdout(&self, data: &[u8]) -> io::Result<()> {
        let mut file = self.stdout_file.lock().unwrap();
        self.log_to_file(&mut file, data)?;

        if self.verbose {
            std::io::stderr().write_all(data).unwrap();
            std::io::stderr().flush().unwrap();
        }

        Ok(())
    }

    /// writes remote stderr to local file as it is streamed. will write copy
    /// to console if `verbose` is true
    fn stderr(&self, data: &[u8]) -> io::Result<()> {
        let mut file = self.stderr_file.lock().unwrap();
        self.log_to_file(&mut file, data)?;

        std::io::stderr().write_all(data).unwrap();
        std::io::stderr().flush().unwrap();

        Ok(())
    }
}
