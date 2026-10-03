use std::collections::HashMap;

use tokio::sync::broadcast;
use zbus::Connection;
use zvariant::Value;

use super::{INTERFACE, PATH, UI_INTERFACE, UI_PATH};
use crate::state::Event;

/// Translate domain events into D-Bus signals on a dedicated task.
pub fn spawn(connection: Connection, mut events: broadcast::Receiver<Event>) {
    tokio::spawn(async move {
        loop {
            match events.recv().await {
                Ok(event) => emit(&connection, event).await,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

async fn emit(connection: &Connection, event: Event) {
    let change = match &event {
        Event::Added(n) => Some(("added", n.id)),
        Event::Updated(n) => Some(("updated", n.id)),
        Event::Removed { id, .. } => Some(("removed", *id)),
        Event::Dnd => Some(("dnd", 0)),
        _ => None,
    };
    match event {
        Event::Removed { id, reason } => {
            let _ = connection
                .emit_signal(
                    None::<&str>,
                    PATH,
                    INTERFACE,
                    "NotificationClosed",
                    &(id, reason),
                )
                .await;
        }
        Event::ActionInvoked { id, key } => {
            let _ = connection
                .emit_signal(
                    None::<&str>,
                    PATH,
                    INTERFACE,
                    "ActionInvoked",
                    &(id, key.as_str()),
                )
                .await;
        }
        Event::Replied { id, text } => {
            let _ = connection
                .emit_signal(
                    None::<&str>,
                    PATH,
                    INTERFACE,
                    "NotificationReplied",
                    &(id, text.as_str()),
                )
                .await;
        }
        Event::InhibitedChanged(value) => emit_properties_changed(connection, value).await,
        Event::Dnd | Event::Added(_) | Event::Updated(_) => {}
    }
    if let Some((kind, id)) = change {
        if let Err(error) = connection
            .emit_signal(None::<&str>, UI_PATH, UI_INTERFACE, "Changed", &(kind, id))
            .await
        {
            log::warn!("could not emit UI change signal: {error}");
        }
    }
}

async fn emit_properties_changed(connection: &Connection, inhibited: bool) {
    let mut changed: HashMap<String, Value<'static>> = HashMap::new();
    changed.insert("Inhibited".to_string(), Value::from(inhibited));
    let invalidated: Vec<String> = Vec::new();

    let _ = connection
        .emit_signal(
            None::<&str>,
            PATH,
            "org.freedesktop.DBus.Properties",
            "PropertiesChanged",
            &(INTERFACE, changed, invalidated),
        )
        .await;
}
