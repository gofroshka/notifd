use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

use tokio::sync::broadcast;

use crate::model::{CloseReason, Notification};

/// Domain events emitted by the state. They feed both the D-Bus signal
/// emitter and the UI socket broadcaster.
#[derive(Clone, Debug)]
pub enum Event {
    /// The user-facing "Do Not Disturb" toggle changed.
    Dnd(bool),
    /// The aggregate `Inhibited` value changed (user DND or an app inhibitor).
    InhibitedChanged(bool),
    Added(Notification),
    Updated(Notification),
    Removed { id: u32, reason: u32 },
    ActionInvoked { id: u32, key: String },
    Replied { id: u32, text: String },
}

struct Inner {
    next_id: u32,
    next_cookie: u32,
    dnd: bool,
    inhibitors: BTreeMap<u32, ()>,
    items: BTreeMap<u32, Notification>,
}

/// Shared, thread-safe notification store. All mutations publish an [`Event`].
pub struct State {
    inner: Mutex<Inner>,
    events: broadcast::Sender<Event>,
    dnd_file: Option<PathBuf>,
}

impl State {
    pub fn new(initial_dnd: bool, dnd_file: Option<PathBuf>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            inner: Mutex::new(Inner {
                next_id: 1,
                next_cookie: 1,
                dnd: initial_dnd,
                inhibitors: BTreeMap::new(),
                items: BTreeMap::new(),
            }),
            events,
            dnd_file,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.events.subscribe()
    }

    pub fn inhibited(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        inner.dnd || !inner.inhibitors.is_empty()
    }

    pub fn snapshot(&self) -> (bool, Vec<Notification>) {
        let inner = self.inner.lock().unwrap();
        (inner.dnd, inner.items.values().cloned().collect())
    }

    pub fn set_dnd(&self, value: bool) {
        let inhibited = {
            let mut inner = self.inner.lock().unwrap();
            if inner.dnd == value {
                return;
            }
            inner.dnd = value;
            inner.dnd || !inner.inhibitors.is_empty()
        };
        self.persist_dnd(value);
        self.publish(Event::Dnd(value));
        self.publish(Event::InhibitedChanged(inhibited));
    }

    pub fn inhibit(&self, reason: String) -> u32 {
        let (cookie, inhibited) = {
            let mut inner = self.inner.lock().unwrap();
            let cookie = inner.next_cookie;
            inner.next_cookie = inner.next_cookie.wrapping_add(1).max(1);
            inner.inhibitors.insert(cookie, ());
            (cookie, true)
        };
        log::info!("inhibited by application ({reason})");
        self.publish(Event::InhibitedChanged(inhibited));
        cookie
    }

    pub fn uninhibit(&self, cookie: u32) {
        let inhibited = {
            let mut inner = self.inner.lock().unwrap();
            if inner.inhibitors.remove(&cookie).is_none() {
                return;
            }
            inner.dnd || !inner.inhibitors.is_empty()
        };
        self.publish(Event::InhibitedChanged(inhibited));
    }

    /// Store a notification, honouring `replaces_id`. Returns the final id and
    /// whether it was freshly added (as opposed to replaced).
    pub fn upsert(&self, replaces_id: u32, mut notification: Notification) -> (u32, bool) {
        let (id, added) = {
            let mut inner = self.inner.lock().unwrap();
            if replaces_id != 0 && inner.items.contains_key(&replaces_id) {
                notification.id = replaces_id;
                inner.items.insert(replaces_id, notification.clone());
                (replaces_id, false)
            } else {
                let id = inner.next_id;
                inner.next_id = inner.next_id.wrapping_add(1).max(1);
                notification.id = id;
                inner.items.insert(id, notification.clone());
                (id, true)
            }
        };
        if added {
            self.publish(Event::Added(notification));
        } else {
            self.publish(Event::Updated(notification));
        }
        (id, added)
    }

    /// Close a notification only if it still carries the given timeout, so a
    /// superseded notification is not closed by a stale timer.
    pub fn close_if_matching(&self, id: u32, expire_timeout: i32, reason: CloseReason) -> bool {
        let matches = self
            .inner
            .lock()
            .unwrap()
            .items
            .get(&id)
            .map(|n| n.expire_timeout == expire_timeout)
            .unwrap_or(false);
        if matches {
            self.close(id, reason)
        } else {
            false
        }
    }

    pub fn get(&self, id: u32) -> Option<Notification> {
        self.inner.lock().unwrap().items.get(&id).cloned()
    }

    pub fn close(&self, id: u32, reason: CloseReason) -> bool {
        let removed = self.inner.lock().unwrap().items.remove(&id);
        if removed.is_some() {
            self.publish(Event::Removed {
                id,
                reason: reason.as_u32(),
            });
            true
        } else {
            false
        }
    }

    pub fn clear(&self, reason: CloseReason) {
        let ids: Vec<u32> = {
            let mut inner = self.inner.lock().unwrap();
            let ids = inner.items.keys().copied().collect();
            inner.items.clear();
            ids
        };
        for id in ids {
            self.publish(Event::Removed {
                id,
                reason: reason.as_u32(),
            });
        }
    }

    /// Emit the primary/button action and close the notification unless it is
    /// resident.
    pub fn invoke(&self, id: u32, key: String) {
        self.publish(Event::ActionInvoked { id, key });
        if let Some(notification) = self.get(id) {
            if !notification.resident {
                self.close(id, CloseReason::Dismissed);
            }
        }
    }

    pub fn reply(&self, id: u32, text: String) {
        self.publish(Event::Replied { id, text });
        self.close(id, CloseReason::Dismissed);
    }

    fn publish(&self, event: Event) {
        let _ = self.events.send(event);
    }

    fn persist_dnd(&self, value: bool) {
        let Some(path) = &self.dnd_file else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Err(error) = std::fs::write(path, if value { "1" } else { "0" }) {
            log::warn!("could not persist DND state: {error}");
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new(false, None)
    }
}
