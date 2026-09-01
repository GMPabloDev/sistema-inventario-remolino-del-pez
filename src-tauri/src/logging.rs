use std::{
    fs::{create_dir_all, File, OpenOptions},
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Clone)]
pub struct Logger {
    file: Arc<Mutex<File>>,
}

impl Logger {
    pub fn new(data_directory: &Path) -> std::io::Result<Self> {
        create_dir_all(data_directory)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(data_directory.join("inventory.log"))?;

        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }

    pub fn log(&self, level: &str, context: &str, message: &str) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or_default();
        let safe_message = message.replace(['\r', '\n'], " ");
        let line = format!("{timestamp} [{level}] {context}: {safe_message}\n");

        if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::Logger;

    #[test]
    fn writes_a_sanitized_local_log_entry() {
        let directory = std::env::temp_dir().join(format!(
            "remolino-pez-log-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after unix epoch")
                .as_nanos()
        ));
        let logger = Logger::new(&directory).expect("logger should initialize");

        logger.log("ERROR", "database", "line one\nline two");

        let contents = fs::read_to_string(directory.join("inventory.log"))
            .expect("log file should be readable");
        assert!(contents.contains("[ERROR] database: line one line two"));
        fs::remove_dir_all(directory).expect("temporary log directory should be removable");
    }
}
