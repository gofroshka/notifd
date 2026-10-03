use std::sync::Arc;

use zbus::{interface, object_server::SignalEmitter};

use crate::model::{CloseReason, Notification, Urgency};
use crate::state::State;

/// One notification on the private, typed D-Bus UI interface.
/// (id, app name, app icon, summary, body, urgency, image path,
///  inline reply, resident, actions)
pub type NotificationData = (
    u32,
    String,
    String,
    String,
    String,
    String,
    String,
    bool,
    bool,
    Vec<(String, String)>,
);

fn notification_data(n: Notification) -> NotificationData {
    (
        n.id,
        n.app_name,
        n.app_icon,
        n.summary,
        n.body,
        match n.urgency {
            Urgency::Low => "low",
            Urgency::Normal => "normal",
            Urgency::Critical => "critical",
        }
        .to_string(),
        n.image_path.unwrap_or_default(),
        n.has_inline_reply,
        n.resident,
        n.actions
            .into_iter()
            .map(|action| (action.identifier, action.text))
            .collect(),
    )
}

/// Private UI interface; the standard org.freedesktop.Notifications API for
/// sending notifications remains unchanged on its original object path.
pub struct Ui {
    pub state: Arc<State>,
}

#[interface(name = "org.gofroshka.Notifd1")]
impl Ui {
    #[zbus(name = "GetSnapshot")]
    fn get_snapshot(&self) -> (bool, Vec<NotificationData>) {
        let (dnd, items) = self.state.snapshot();
        (dnd, items.into_iter().map(notification_data).collect())
    }

    #[zbus(name = "SetDnd")]
    fn set_dnd(&self, value: bool) {
        self.state.set_dnd(value);
    }

    #[zbus(name = "Dismiss")]
    fn dismiss(&self, id: u32) {
        self.state.close(id, CloseReason::Dismissed);
    }

    #[zbus(name = "Invoke")]
    fn invoke(&self, id: u32, key: &str) {
        self.state.invoke(id, key.to_string());
    }

    #[zbus(name = "Reply")]
    fn reply(&self, id: u32, text: &str) {
        self.state.reply(id, text.to_string());
    }

    #[zbus(name = "Clear")]
    fn clear(&self) {
        self.state.clear(CloseReason::Dismissed);
    }

    /// An invalidation signal: clients fetch an atomic snapshot after changes.
    /// `kind` is added, updated, removed or dnd; `id` is 0 for dnd.
    #[zbus(signal, name = "Changed")]
    async fn changed(emitter: &SignalEmitter<'_>, kind: &str, id: u32) -> zbus::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Action, SoundHint};

    #[test]
    fn snapshot_preserves_notification_fields_and_actions() {
        let state = Arc::new(State::default());
        state.upsert(
            0,
            Notification {
                id: 0,
                app_name: "Chat".into(),
                app_icon: "chat".into(),
                summary: "Hello".into(),
                body: "World".into(),
                urgency: Urgency::Critical,
                category: None,
                desktop_entry: None,
                actions: vec![Action {
                    identifier: "default".into(),
                    text: "Open".into(),
                }],
                has_inline_reply: true,
                inline_reply_placeholder: None,
                image_path: Some("/tmp/image.png".into()),
                sound: SoundHint::Default,
                expire_timeout: 0,
                resident: true,
                transient: false,
            },
        );
        let (dnd, items) = (Ui { state }).get_snapshot();
        assert!(!dnd);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].0, 1);
        assert_eq!(items[0].5, "critical");
        assert_eq!(items[0].6, "/tmp/image.png");
        assert!(items[0].7);
        assert!(items[0].8);
        assert_eq!(items[0].9, [("default".into(), "Open".into())]);
    }
}
