use std::path::PathBuf;

pub const TELEMOST_URL: &str = "https://telemost.360.yandex.ru";
#[cfg(target_os = "macos")]
pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15";

pub fn runtime_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".runtime")
}
