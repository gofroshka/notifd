use serde::{Deserialize, Serialize};

use crate::model::Notification;
use crate::state::Event;

/// Messages the daemon pushes to the UI over the Unix socket (JSON lines).
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    Snapshot {
        dnd: bool,
        notifications: Vec<Notification>,
    },
    Dnd {
        value: bool,
    },
    Added {
        notification: Notification,
    },
    Updated {
        notification: Notification,
    },
    Removed {
        id: u32,
        reason: u32,
    },
}

impl ServerEvent {
    /// Only events relevant to the UI survive; D-Bus signal events are dropped.
    pub fn from_event(event: Event) -> Option<Self> {
        match event {
            Event::Dnd(value) => Some(Self::Dnd { value }),
            Event::Added(notification) => Some(Self::Added { notification }),
            Event::Updated(notification) => Some(Self::Updated { notification }),
            Event::Removed { id, reason } => Some(Self::Removed { id, reason }),
            Event::InhibitedChanged(_) | Event::ActionInvoked { .. } | Event::Replied { .. } => {
                None
            }
        }
    }
}

/// Commands the UI sends back to the daemon (JSON lines).
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientCommand {
    SetDnd { value: bool },
    Dismiss { id: u32 },
    Invoke { id: u32, key: String },
    Reply { id: u32, text: String },
    Clear,
}
