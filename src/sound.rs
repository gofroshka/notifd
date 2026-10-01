use std::path::PathBuf;

use tokio::process::Command;

use crate::model::SoundHint;

/// Play the sound for a notification. The client's own sound (a `sound-file`
/// or a themeable `sound-name`) always wins; the configured standard sound is
/// only used as the fallback when the client asks for the default one.
pub async fn play(standard: Option<&str>, hint: &SoundHint) {
    let request = match hint {
        SoundHint::Suppress => return,
        SoundHint::File(file) => Request::File(file.clone()),
        SoundHint::Name(name) => Request::Name(name.clone()),
        SoundHint::Default => match standard {
            Some(sound) => Request::parse(sound),
            None => return,
        },
    };

    log::debug!("playing notification sound: {}", request.describe());

    if request.run().await {
        return;
    }

    if let Some(path) = request.fallback_file() {
        if run("paplay", &[path.as_str()]).await {
            return;
        }
    }
    log::warn!("could not play notification sound: no working player found");
}

enum Request {
    Name(String),
    File(String),
}

impl Request {
    fn describe(&self) -> String {
        match self {
            Self::File(file) => format!("file {file}"),
            Self::Name(name) => format!("name {name}"),
        }
    }

    fn parse(value: &str) -> Self {
        if value.starts_with('/') {
            Self::File(value.to_string())
        } else {
            Self::Name(value.to_string())
        }
    }

    /// Preferred player: libcanberra, which tags the stream `media.role=event`.
    async fn run(&self) -> bool {
        match self {
            Self::Name(name) => run("canberra-gtk-play", &["-i", name]).await,
            Self::File(file) => run("canberra-gtk-play", &["-f", file]).await,
        }
    }

    fn fallback_file(&self) -> Option<String> {
        match self {
            Self::File(file) => Some(file.clone()),
            Self::Name(name) => resolve_theme_sound(name).map(|p| p.to_string_lossy().into_owned()),
        }
    }
}

/// Spawn a player and reap it in the background so we never block notification
/// handling and never leak zombies.
async fn run(program: &str, args: &[&str]) -> bool {
    match Command::new(program).args(args).spawn() {
        Ok(mut child) => {
            tokio::spawn(async move {
                let _ = child.wait().await;
            });
            true
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(error) => {
            log::warn!("failed to run {program}: {error}");
            false
        }
    }
}

fn resolve_theme_sound(name: &str) -> Option<PathBuf> {
    let mut roots = Vec::new();
    if let Some(data_home) = std::env::var_os("XDG_DATA_HOME") {
        roots.push(PathBuf::from(data_home).join("sounds"));
    }
    let data_dirs = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    roots.extend(
        data_dirs
            .split(':')
            .map(|dir| PathBuf::from(dir).join("sounds")),
    );
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join(".local/share/sounds"));
    }

    for root in roots {
        for theme in ["freedesktop", "Freedesktop", "default"] {
            for extension in ["oga", "ogg"] {
                let candidate = root
                    .join(theme)
                    .join("stereo")
                    .join(format!("{name}.{extension}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}
