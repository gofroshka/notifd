use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use zbus::interface;
use zvariant::OwnedValue;

use crate::config::Config;
use crate::hints::ParsedHints;
use crate::model::{Action, CloseReason, Notification};
use crate::state::State;
use crate::{image_cache, sound};

/// Implementation of `org.freedesktop.Notifications`.
pub struct Notifications {
    pub state: Arc<State>,
    pub config: Config,
}

#[interface(name = "org.freedesktop.Notifications")]
impl Notifications {
    #[zbus(name = "GetCapabilities")]
    async fn get_capabilities(&self) -> Vec<String> {
        ["actions", "body", "icon-static", "persistence", "sound"]
            .iter()
            .map(|cap| cap.to_string())
            .collect()
    }

    #[zbus(name = "GetServerInformation")]
    async fn get_server_information(&self) -> (String, String, String, String) {
        (
            "notifd".to_string(),
            "notifd".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
            "1.2".to_string(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[zbus(name = "Notify")]
    async fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        let parsed = ParsedHints::parse(&hints);
        let (actions, has_inline_reply) = parse_actions(&actions);

        let notification = Notification {
            id: 0,
            app_name: app_name.to_string(),
            app_icon: app_icon.to_string(),
            summary: summary.to_string(),
            body: body.to_string(),
            urgency: parsed.urgency,
            category: parsed.category.clone(),
            desktop_entry: parsed.desktop_entry.clone(),
            actions,
            has_inline_reply,
            inline_reply_placeholder: None,
            image_path: self.resolve_image(&parsed),
            sound: parsed.sound.clone(),
            expire_timeout,
            resident: parsed.resident,
            transient: parsed.transient,
        };

        let (id, added) = self.state.upsert(replaces_id, notification);

        if added && !self.state.inhibited() {
            sound::play(self.config.standard_sound.as_deref(), &parsed.sound).await;
        }

        if added && expire_timeout > 0 {
            self.schedule_expiry(id, expire_timeout);
        }

        id
    }

    #[zbus(name = "CloseNotification")]
    async fn close_notification(&self, id: u32) {
        self.state.close(id, CloseReason::ClosedByCall);
    }

    #[zbus(name = "Inhibit")]
    async fn inhibit(
        &self,
        _desktop_entry: &str,
        reason: &str,
        _hints: HashMap<String, OwnedValue>,
    ) -> u32 {
        self.state.inhibit(reason.to_string())
    }

    #[zbus(name = "UnInhibit")]
    async fn uninhibit(&self, cookie: u32) {
        self.state.uninhibit(cookie);
    }

    #[zbus(property, name = "Inhibited")]
    async fn inhibited(&self) -> bool {
        self.state.inhibited()
    }
}

impl Notifications {
    /// Auto-close a notification once its `expire_timeout` elapses.
    fn schedule_expiry(&self, id: u32, expire_timeout: i32) {
        let state = self.state.clone();
        tokio::spawn(async move {
            let delay = std::time::Duration::from_millis(expire_timeout as u64);
            tokio::time::sleep(delay).await;
            state.close_if_matching(id, expire_timeout, CloseReason::Expired);
        });
    }

    /// Cache `image-data` to a file and fall back to an explicit `image-path`.
    fn resolve_image(&self, parsed: &ParsedHints) -> Option<String> {
        static NEXT_IMAGE: AtomicU32 = AtomicU32::new(1);

        if let Some(data) = &parsed.image_data {
            let name = NEXT_IMAGE.fetch_add(1, Ordering::Relaxed);
            if let Some(path) = image_cache::cache(data, &self.config.cache_dir, name) {
                return Some(path.to_string_lossy().into_owned());
            }
        }
        parsed.image_path.clone()
    }
}

/// Pair the flat `[id, text, ...]` action list and detect an inline reply.
fn parse_actions(raw: &[String]) -> (Vec<Action>, bool) {
    let mut actions = Vec::new();
    let mut has_inline_reply = false;

    for pair in raw.chunks_exact(2) {
        if pair[0] == "inline-reply" {
            has_inline_reply = true;
            continue;
        }
        actions.push(Action {
            identifier: pair[0].clone(),
            text: pair[1].clone(),
        });
    }

    (actions, has_inline_reply)
}
