use std::path::PathBuf;

/// Runtime configuration, sourced entirely from environment variables so the
/// systemd unit stays declarative.
#[derive(Clone, Debug)]
pub struct Config {
    pub cache_dir: PathBuf,
    /// Where the user's DND toggle is persisted across restarts.
    pub dnd_file: PathBuf,
    /// When set, every notification plays this sound instead of the app's
    /// requested one (path or XDG sound name). `None` honours the client hints.
    pub standard_sound: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let cache_root = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        let state_root = std::env::var_os("XDG_STATE_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state"))
            })
            .unwrap_or_else(|| PathBuf::from("/tmp"));

        Self {
            cache_dir: cache_root.join("notifd"),
            dnd_file: state_root.join("notifd/dnd"),
            standard_sound: parse_standard_sound(),
        }
    }
}

/// Defaults to the freedesktop `message-new-instant` event sound. Set
/// `NOTIFD_STANDARD_SOUND` to a path/name, or to an empty string to let each
/// application pick its own sound.
fn parse_standard_sound() -> Option<String> {
    match std::env::var("NOTIFD_STANDARD_SOUND") {
        Ok(value) if value.is_empty() => None,
        Ok(value) => Some(value),
        Err(_) => Some("message-new-instant".to_string()),
    }
}
