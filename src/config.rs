use std::path::PathBuf;

const DEFAULT_HASH: &str = "$2b$12$G95bkoHmmCqLvabUSSsNc.U/lcHJzy2/ncG2ZgVja8SoKdS50/8Fi";

pub const FOCUS_POLL_MS: u64 = 500;

pub const ERROR_FLASH_FRAMES: u32 = 30;

pub fn verify_password(input: &str) -> bool {
    let hash = load_password_hash();
    bcrypt::verify(input, &hash).unwrap_or(false)
}

pub fn set_password(password: &str) -> Result<(), String> {
    let hash = bcrypt::hash(password, 12).map_err(|e| format!("hash error: {e}"))?;
    save_password_hash(&hash).map_err(|e| format!("save error: {e}"))
}

fn load_password_hash() -> String {
    if let Some(path) = config_file_path() {
        if let Ok(contents) = std::fs::read_to_string(path) {
            let h = contents.trim();
            if !h.is_empty() {
                return h.to_string();
            }
        }
    }
    DEFAULT_HASH.to_string()
}

fn save_password_hash(hash: &str) -> std::io::Result<()> {
    let path = config_file_path().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "cannot determine home directory",
        )
    })?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, hash)
}

fn config_file_path() -> Option<PathBuf> {
    let home = std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())?;
    Some(PathBuf::from(home).join(".active-lock").join("password.hash"))
}
