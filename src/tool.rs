#[cfg(any(
    feature = "mail-sendmail",
    feature = "mail-smtp",
    feature = "mail-file",
    feature = "mail-db",
    feature = "session-memory",
    feature = "session-file",
    feature = "session-db"
))]
use chrono::Local;

#[cfg(any(
    feature = "mail-sendmail",
    feature = "mail-smtp",
    feature = "mail-file",
    feature = "mail-db",
    feature = "session-memory",
    feature = "session-file",
    feature = "session-db"
))]
use ring::rand::{SecureRandom, SystemRandom};

#[cfg(any(
    feature = "mail-sendmail",
    feature = "mail-smtp",
    feature = "mail-file",
    feature = "mail-db",
    feature = "session-memory",
    feature = "session-file",
    feature = "session-db"
))]
use sha3::{Digest, Sha3_512};

#[cfg(any(
    feature = "mail-sendmail",
    feature = "mail-smtp",
    feature = "mail-file",
    feature = "mail-db",
    feature = "session-memory",
    feature = "session-file",
    feature = "session-db"
))]
pub(crate) fn generate_uuid() -> String {
    fn shuffle_string(s: &str) -> String {
        let mut chars: Vec<char> = s.chars().collect();
        let len = chars.len();
        let rng = SystemRandom::new();

        for i in (1..len).rev() {
            let mut buf = [0u8; 8];
            let _ = rng.fill(&mut buf);
            let rand_index = (u64::from_ne_bytes(buf) % (i as u64 + 1)) as usize;
            chars.swap(i, rand_index);
        }
        for c in chars.iter_mut() {
            let mut buf = [0u8; 1];
            let _ = rng.fill(&mut buf);
            if buf[0] % 2 == 0 {
                *c = c.to_ascii_uppercase();
            }
        }
        let mut str: String = chars.into_iter().collect();
        str.truncate(32);
        str
    }

    let rng = SystemRandom::new();
    let mut random_bytes = [0u8; 32];

    if rng.fill(&mut random_bytes).is_err() {
        let time = Local::now().format("%Y%m%d%H%M%S%9f").to_string() + "!@#$%^&*_";
        let time = shuffle_string(&time);
        random_bytes.copy_from_slice(time.as_bytes());
    }
    let mut hasher = Sha3_512::new();
    hasher.update(random_bytes);
    format!("{:#x}", hasher.finalize())
}
