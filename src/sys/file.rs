use std::{
    env,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use chrono::Local;
use tokio::{fs::File, io::AsyncWriteExt};

use crate::fnv1a_64;

use super::log::Log;

/// Empty struct for working temp file
pub struct TempFile;

impl TempFile {
    /// Creates a unique new temporary file name
    pub fn new_name() -> PathBuf {
        let mut temp_dir = env::temp_dir();

        // Generate a unique file name
        let time = SystemTime::now();
        let file_name = match time.duration_since(UNIX_EPOCH) {
            Ok(epoch) => format!("tiny_{}_{}.tmp", epoch.as_secs(), epoch.subsec_nanos()),
            Err(e) => {
                Log::warning(2005, Some(e.to_string()));
                format!(
                    "tiny_{}.tmp",
                    fnv1a_64(&Local::now().format("%Y.%m.%d %H:%M:%S%.9f").to_string())
                )
            }
        };
        temp_dir.push(file_name);

        temp_dir
    }

    /// Write data to the temporary file
    pub async fn write(path: &PathBuf, data: &[u8]) -> Result<(), ()> {
        let mut file = match File::create(path).await {
            Ok(file) => file,
            Err(e) => {
                Log::warning(2002, Some(format!("{}. Error: {}", path.display(), e)));
                return Err(());
            }
        };
        let len = match file.write(data).await {
            Ok(len) => len,
            Err(e) => {
                Log::warning(2003, Some(format!("{}. Error: {}", path.display(), e)));
                return Err(());
            }
        };
        if data.len() == len {
            Ok(())
        } else {
            Log::warning(
                2004,
                Some(format!(
                    "{}. Written len: {} ({})",
                    path.display(),
                    len,
                    data.len()
                )),
            );
            Err(())
        }
    }
}
