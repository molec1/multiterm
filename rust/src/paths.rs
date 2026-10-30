//! Filesystem locations and path display.

use std::path::{Path, PathBuf};

pub const APP_ID: &str = "org.local.Multiterm";
pub const ICON_NAME: &str = "org.local.Multiterm";

const ICON_SVG: &[u8] = include_bytes!("../../data/icons/hicolor/scalable/apps/org.local.Multiterm.svg");

/// Write the app icon and return its path. Benchmarks keep it in the data dir.
pub fn install_app_icon() -> PathBuf {
    let dir = if std::env::var_os("MULTITERM_BENCH").is_some() {
        data_dir()
    } else if let Some(base) = std::env::var_os("XDG_DATA_HOME") {
        PathBuf::from(base).join("icons/hicolor/scalable/apps")
    } else {
        home().join(".local/share/icons/hicolor/scalable/apps")
    };
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join(format!("{ICON_NAME}.svg"));
    let _ = std::fs::write(&path, ICON_SVG);
    path
}

pub fn data_dir() -> PathBuf {
    if let Some(override_dir) = std::env::var_os("MULTITERM_DATA") {
        return PathBuf::from(override_dir);
    }
    if let Some(base) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(base).join("multiterm");
    }
    home().join(".local/share/multiterm")
}

pub fn ensure_data() -> PathBuf {
    let root = data_dir();
    let history = root.join("history");
    let _ = std::fs::create_dir_all(&history);
    let _ = std::fs::set_permissions(&root, mode(0o700));
    let _ = std::fs::set_permissions(&history, mode(0o700));
    root
}

pub fn display_path(path: &str) -> String {
    let home = home();
    let home = home.to_string_lossy();
    if path == home {
        return "~".to_string();
    }
    let prefix = format!("{home}/");
    if let Some(rest) = path.strip_prefix(&prefix) {
        return format!("~/{rest}");
    }
    path.to_string()
}

pub fn folder_uri(path: &str) -> String {
    let absolute = Path::new(path).canonicalize().unwrap_or_else(|_| PathBuf::from(path));
    format!("file://{}", absolute.display())
}

pub fn existing_directory(path: &str) -> String {
    if !path.is_empty() && Path::new(path).is_dir() {
        return path.to_string();
    }
    home().to_string_lossy().into_owned()
}

pub fn pid_cwd(pid: Option<i32>) -> Option<String> {
    let pid = pid?;
    if pid <= 0 {
        return None;
    }
    let target = std::fs::read_link(format!("/proc/{pid}/cwd")).ok()?;
    if target.is_dir() {
        Some(target.to_string_lossy().into_owned())
    } else {
        None
    }
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/tmp"))
}

pub fn new_id() -> String {
    let mut bytes = [0u8; 16];
    if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = file.read_exact(&mut bytes);
    }
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn mode(bits: u32) -> std::fs::Permissions {
    use std::os::unix::fs::PermissionsExt;
    std::fs::Permissions::from_mode(bits)
}
