use serde::{Deserialize, Serialize};

/// Notification urgency, as defined by the Desktop Notifications Specification.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Urgency {
    Low,
    #[default]
    Normal,
    Critical,
}

impl Urgency {
    pub fn from_hint(value: u8) -> Self {
        match value {
            0 => Self::Low,
            2 => Self::Critical,
            _ => Self::Normal,
        }
    }
}

/// A single action offered by a notification. `default` is the primary action.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Action {
    pub identifier: String,
    pub text: String,
}

/// What the client wants the daemon to do about sound.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundHint {
    /// No sound requested, use the default.
    #[default]
    Default,
    /// Client plays its own sound; the daemon must stay silent.
    Suppress,
    /// Play the sound from this file.
    File(String),
    /// Play the named sound from the XDG sound theme.
    Name(String),
}

/// A notification tracked by the daemon and mirrored to the UI.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Notification {
    pub id: u32,
    pub app_name: String,
    pub app_icon: String,
    pub summary: String,
    pub body: String,
    pub urgency: Urgency,
    pub category: Option<String>,
    pub desktop_entry: Option<String>,
    pub actions: Vec<Action>,
    pub has_inline_reply: bool,
    pub inline_reply_placeholder: Option<String>,
    pub image_path: Option<String>,
    pub sound: SoundHint,
    pub expire_timeout: i32,
    pub resident: bool,
    pub transient: bool,
}

/// Why a notification was closed, matching the spec's `NotificationClosed` reasons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseReason {
    Expired = 1,
    Dismissed = 2,
    ClosedByCall = 3,
}

impl CloseReason {
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}
