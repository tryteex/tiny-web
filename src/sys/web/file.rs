use std::{
    env,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::Local;

use tokio::{fs::File, io::AsyncWriteExt};

use crate::{fnv1a_64, log};

/// Empty struct for working temp file
pub(crate) struct TempFile;

impl TempFile {
    /// Creates a unique new temporary file name
    pub fn new_name() -> PathBuf {
        let mut temp_dir = env::temp_dir();

        // Generate a unique file name
        let time = SystemTime::now();
        let file_name = match time.duration_since(UNIX_EPOCH) {
            Ok(epoch) => format!("tiny_{}_{}.tmp", epoch.as_secs(), epoch.subsec_nanos()),
            Err(_e) => {
                log!(warning, 0, "{}", _e);
                format!("tiny_{}.tmp", fnv1a_64(Local::now().format("%Y.%m.%d %H:%M:%S%.9f").to_string().as_bytes()))
            }
        };
        temp_dir.push(file_name);

        temp_dir
    }

    /// Write data to the temporary file
    pub async fn write(path: &PathBuf, data: &[u8]) -> Result<(), ()> {
        let mut file = match File::create(path).await {
            Ok(file) => file,
            Err(_e) => {
                log!(warning, 0, "{}. Error: {}", path.display(), _e);
                return Err(());
            }
        };
        let len = match file.write(data).await {
            Ok(len) => len,
            Err(_e) => {
                log!(warning, 0, "{}. Error: {}", path.display(), _e);
                return Err(());
            }
        };
        if data.len() == len {
            Ok(())
        } else {
            log!(warning, 0, "{}. Written len: {} ({})", path.display(), len, data.len());
            Err(())
        }
    }
}
