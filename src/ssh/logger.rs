use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct ConnectionLogger {
    stdout_file: Arc<Mutex<File>>,
    stderr_file: Arc<Mutex<File>>,
    host_id: String,
}

impl ConnectionLogger {
    pub fn new(host_id: &str, log_dir: &Path) -> io::Result<Self> {
        std::fs::create_dir_all(log_dir)?;

        let stdout_path = log_dir.join(format!("{}_stdout.log", host_id));
        let stderr_path = log_dir.join(format!("{}_stderr.log", host_id));

        let stdout_file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(stdout_path)?;

        let stderr_file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(stderr_path)?;

        Ok(Self {
            stdout_file: Arc::new(Mutex::new(stdout_file)),
            stderr_file: Arc::new(Mutex::new(stderr_file)),
            host_id: host_id.to_string(),
        })
    }

    pub fn log_stdout(&self, data: &[u8]) -> io::Result<()> {
        let timestamp = chrono::Local::now()
            .format("[%Y-%m-%d %H:%M:%S] ")
            .to_string();
        let mut file = self.stdout_file.lock().unwrap();
        file.write_all(timestamp.as_bytes())?;
        file.write_all(data)?;
        if !data.ends_with(b"\n") {
            file.write_all(b"\n")?;
        }
        file.flush()
    }

    pub fn log_stderr(&self, data: &[u8]) -> io::Result<()> {
        let timestamp = chrono::Local::now()
            .format("[%Y-%m-%d %H:%M:%S] ")
            .to_string();
        let mut file = self.stderr_file.lock().unwrap();
        file.write_all(timestamp.as_bytes())?;
        file.write_all(data)?;
        if !data.ends_with(b"\n") {
            file.write_all(b"\n")?;
        }
        file.flush()
    }

    pub fn host_id(&self) -> &str {
        &self.host_id
    }
}
