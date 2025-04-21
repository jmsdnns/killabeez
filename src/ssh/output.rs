use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

/// A trait focused on managing stdout and stderr
pub trait OutputHandler: Send + Sync {
    fn host_id(&self) -> &str;
    fn stdout(&self, data: &[u8]) -> io::Result<()>;
    fn stderr(&self, data: &[u8]) -> io::Result<()>;
    fn update_command(&self, command: &str) -> String {
        String::from(command)
    }
}

/// Basic console output
pub struct ConsoleLogger {
    host_id: String,
}
impl ConsoleLogger {
    pub fn new(host_id: &str) -> io::Result<Self> {
        Ok(Self {
            host_id: host_id.to_string(),
        })
    }
}

impl OutputHandler for ConsoleLogger {
    fn host_id(&self) -> &str {
        &self.host_id
    }

    fn stdout(&self, data: &[u8]) -> io::Result<()> {
        std::io::stdout().write_all(data).unwrap();
        std::io::stdout().flush().unwrap();
        Ok(())
    }

    fn stderr(&self, data: &[u8]) -> io::Result<()> {
        std::io::stderr().write_all(data).unwrap();
        std::io::stderr().flush().unwrap();
        Ok(())
    }
}

/// Write stdout & stderr to the remote filesystem, minimizing chat between
/// local machine and remotes
pub struct RemoteFiles {
    host_id: String,
    out_path: PathBuf,
    err_path: PathBuf,
}
impl RemoteFiles {
    pub fn new(
        host_id: &str,
        out_path: Option<PathBuf>,
        err_path: Option<PathBuf>,
    ) -> io::Result<Self> {
        let out_path = match out_path {
            Some(path) => path,
            None => PathBuf::from("stdout.log"),
        };

        let err_path = match err_path {
            Some(path) => path,
            None => PathBuf::from("stderr.log"),
        };

        Ok(Self {
            host_id: host_id.to_string(),
            out_path,
            err_path,
        })
    }
}

impl OutputHandler for RemoteFiles {
    fn host_id(&self) -> &str {
        &self.host_id
    }

    fn stdout(&self, data: &[u8]) -> io::Result<()> {
        Ok(())
    }

    fn stderr(&self, data: &[u8]) -> io::Result<()> {
        Ok(())
    }

    fn update_command(&self, command: &str) -> String {
        let redirect_out = format!(
            r#"awk '{{ print strftime("[%Y-%m-%d %H:%M:%S] "), $0 }}' > {}"#,
            &self.out_path.display()
        );
        let redirect_err = format!(
            r#"awk '{{ print strftime("[%Y-%m-%d %H:%M:%S] "), $0 }}' > {}"#,
            &self.err_path.display()
        );

        // NOTE: logs are appended with `>>`
        format!("{} >> >({}) 2>> >({})", command, redirect_out, redirect_err)
    }
}

/// Write remote stdout & stderr to local files as it is streamed from each machine
pub struct StreamLogger {
    host_id: String,
    stdout_file: Arc<Mutex<File>>,
    stderr_file: Arc<Mutex<File>>,
}
impl StreamLogger {
    pub fn new(host_id: &str, log_dir: &Path) -> io::Result<Self> {
        std::fs::create_dir_all(log_dir)?;

        let stdout_path = log_dir.join(format!("{}_stdout.log", host_id));
        let stderr_path = log_dir.join(format!("{}_stderr.log", host_id));

        let stdout_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(stdout_path)?;

        let stderr_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(stderr_path)?;

        Ok(Self {
            stdout_file: Arc::new(Mutex::new(stdout_file)),
            stderr_file: Arc::new(Mutex::new(stderr_file)),
            host_id: host_id.to_string(),
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

impl OutputHandler for StreamLogger {
    fn host_id(&self) -> &str {
        &self.host_id
    }

    fn stdout(&self, data: &[u8]) -> io::Result<()> {
        let mut file = self.stdout_file.lock().unwrap();
        self.log_to_file(&mut file, data)
    }

    fn stderr(&self, data: &[u8]) -> io::Result<()> {
        let mut file = self.stderr_file.lock().unwrap();
        self.log_to_file(&mut file, data)
    }
}
