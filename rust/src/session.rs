//! Saved window: which tabs are open, how they are grouped, and their folders.

use std::collections::HashSet;
use std::path::Path;

use serde_json::{Value, json};

use crate::paths::{data_dir, ensure_data};

pub const SESSION_VERSION: u64 = 2;
const IMPLICIT_GROUP_NAMES: [&str; 2] = ["", "Tabs"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabRecord {
    pub id: String,
    pub cwd: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupRecord {
    pub id: String,
    pub name: String,
    pub tabs: Vec<TabRecord>,
    pub active: i64,
    pub named: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Session {
    pub groups: Vec<GroupRecord>,
    pub active_group: i64,
    pub width: i64,
    pub height: i64,
}

impl Session {
    pub fn tabs(&self) -> impl Iterator<Item = &TabRecord> {
        self.groups.iter().flat_map(|group| group.tabs.iter())
    }

    pub fn window_size(&self) -> Option<(i32, i32)> {
        if self.width > 200 && self.height > 150 {
            Some((self.width as i32, self.height as i32))
        } else {
            None
        }
    }
}

pub fn session_path() -> std::path::PathBuf {
    data_dir().join("session.json")
}

pub fn load_session() -> Option<Session> {
    let raw: Value = serde_json::from_str(&std::fs::read_to_string(session_path()).ok()?).ok()?;
    let object = raw.as_object()?;
    let groups = if let Some(items) = object.get("groups").and_then(Value::as_array) {
        parse_groups(items)
    } else if object.get("tabs").and_then(Value::as_array).is_some() {
        vec![flat_group(&raw)]
    } else {
        return None;
    };
    Some(Session {
        groups,
        active_group: int_of(object.get("active_group"), 0),
        width: int_of(object.get("width"), 1000),
        height: int_of(object.get("height"), 640),
    })
}

pub fn save_session(session: &Session) {
    ensure_data();
    let path = session_path();
    let temporary = path.with_extension("json.tmp");
    let payload = json!({
        "version": SESSION_VERSION,
        "active_group": session.active_group,
        "width": session.width,
        "height": session.height,
        "groups": session.groups.iter().map(|group| json!({
            "id": group.id,
            "name": group.name,
            "named": group.named,
            "active": group.active,
            "tabs": group.tabs.iter().map(|tab| json!({
                "id": tab.id,
                "cwd": tab.cwd,
                "title": tab.title,
            })).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    });
    let text = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string());
    if std::fs::write(&temporary, format!("{text}\n")).is_err() {
        return;
    }
    let _ = std::fs::set_permissions(&temporary, {
        use std::os::unix::fs::PermissionsExt;
        std::fs::Permissions::from_mode(0o600)
    });
    let _ = std::fs::rename(temporary, path);
}

pub fn gc_history(keep_ids: &HashSet<String>) {
    let history = data_dir().join("history");
    let Ok(entries) = std::fs::read_dir(history) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if let Some(stem) = name.strip_suffix(".ready") {
            if !keep_ids.contains(stem) {
                let _ = std::fs::remove_file(entry.path());
            }
            continue;
        }
        if entry.path().is_file() && !keep_ids.contains(name.as_ref()) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn parse_groups(items: &[Value]) -> Vec<GroupRecord> {
    let pending: Vec<(GroupRecord, Option<bool>)> = items
        .iter()
        .filter_map(|item| {
            let object = item.as_object()?;
            let group = parse_group(object)?;
            let explicit = object.get("named").and_then(Value::as_bool);
            Some((group, explicit))
        })
        .collect();
    let only = pending.len() == 1;
    pending
        .into_iter()
        .map(|(mut group, explicit)| {
            if let Some(named) = explicit {
                group.named = named;
            } else if only && IMPLICIT_GROUP_NAMES.contains(&group.name.as_str()) {
                group.named = false;
                group.name.clear();
            } else {
                group.named = true;
            }
            group
        })
        .collect()
}

fn parse_group(item: &serde_json::Map<String, Value>) -> Option<GroupRecord> {
    let id = item.get("id")?.as_str()?;
    if !safe_id(id) {
        return None;
    }
    Some(GroupRecord {
        id: id.to_string(),
        name: item.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
        tabs: parse_tabs(item.get("tabs").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])),
        active: int_of(item.get("active"), 0),
        named: false,
    })
}

fn flat_group(raw: &Value) -> GroupRecord {
    GroupRecord {
        id: "tabs".to_string(),
        name: String::new(),
        named: false,
        tabs: parse_tabs(raw.get("tabs").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])),
        active: int_of(raw.get("active"), 0),
    }
}

fn parse_tabs(items: &[Value]) -> Vec<TabRecord> {
    items
        .iter()
        .filter_map(|item| {
            let object = item.as_object()?;
            let id = object.get("id")?.as_str()?;
            if !safe_id(id) {
                return None;
            }
            Some(TabRecord {
                id: id.to_string(),
                cwd: object.get("cwd").and_then(Value::as_str).unwrap_or("").to_string(),
                title: object
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            })
        })
        .collect()
}

fn int_of(value: Option<&Value>, fallback: i64) -> i64 {
    value.and_then(Value::as_i64).unwrap_or(fallback)
}

fn safe_id(value: &str) -> bool {
    !value.is_empty() && value != "." && value != ".." && !value.contains('/') && !value.contains('\\')
}

pub fn history_path(id: &str) -> std::path::PathBuf {
    data_dir().join("history").join(id)
}

pub fn ready_path(history: &Path) -> std::path::PathBuf {
    let mut name = history.file_name().unwrap_or_default().to_os_string();
    name.push(".ready");
    history.with_file_name(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static LOCK: Mutex<()> = Mutex::new(());

    fn isolate(dir: &Path) {
        unsafe { std::env::set_var("MULTITERM_DATA", dir) };
        let _ = std::fs::remove_dir_all(dir);
        ensure_data();
    }

    #[test]
    fn round_trip_keeps_named_groups() {
        let _guard = LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("multiterm-rs-session-round");
        isolate(&dir);
        let session = Session {
            groups: vec![
                GroupRecord {
                    id: "main".into(),
                    name: "Work".into(),
                    named: true,
                    active: 1,
                    tabs: vec![
                        TabRecord { id: "one".into(), cwd: "/tmp/one".into(), title: "api".into() },
                        TabRecord { id: "two".into(), cwd: "/tmp/two".into(), title: String::new() },
                    ],
                },
                GroupRecord {
                    id: "side".into(),
                    name: "Ops".into(),
                    named: true,
                    active: 0,
                    tabs: vec![TabRecord { id: "three".into(), cwd: "/tmp/three".into(), title: String::new() }],
                },
            ],
            active_group: 1,
            width: 800,
            height: 600,
        };
        save_session(&session);
        let loaded = load_session().expect("session");
        assert_eq!(loaded, session);
        assert_eq!(loaded.window_size(), Some((800, 600)));
    }

    #[test]
    fn version_1_session_is_loose_tabs() {
        let _guard = LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("multiterm-rs-session-v1");
        isolate(&dir);
        let payload = json!({
            "version": 1,
            "active": 0,
            "width": 900,
            "height": 500,
            "tabs": [{"id": "kept", "cwd": "/tmp/kept", "title": "  api  "}],
        });
        std::fs::write(session_path(), payload.to_string()).unwrap();
        let loaded = load_session().unwrap();
        assert_eq!(loaded.groups.len(), 1);
        assert_eq!(loaded.groups[0].name, "");
        assert!(!loaded.groups[0].named);
        assert_eq!(loaded.groups[0].tabs[0].title, "api");
    }

    #[test]
    fn lone_tabs_group_is_loose_and_explicit_flag_wins() {
        let _guard = LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join("multiterm-rs-session-flag");
        isolate(&dir);
        let payload = json!({
            "version": 2,
            "groups": [{"id": "tabs", "name": "Tabs", "tabs": [{"id": "kept", "cwd": "/tmp"}]}],
        });
        std::fs::write(session_path(), payload.to_string()).unwrap();
        let loaded = load_session().unwrap();
        assert!(!loaded.groups[0].named);
        assert_eq!(loaded.groups[0].name, "");

        let payload = json!({
            "version": 2,
            "groups": [{"id": "tabs", "name": "Tabs", "named": true, "tabs": [{"id": "kept", "cwd": "/tmp"}]}],
        });
        std::fs::write(session_path(), payload.to_string()).unwrap();
        let loaded = load_session().unwrap();
        assert!(loaded.groups[0].named);
        assert_eq!(loaded.groups[0].name, "Tabs");
    }
}
