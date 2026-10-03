mod interface;
mod signals;
mod ui;

use std::sync::Arc;

use anyhow::Result;
use zbus::Connection;

use crate::config::Config;
use crate::state::State;

pub const PATH: &str = "/org/freedesktop/Notifications";
pub const INTERFACE: &str = "org.freedesktop.Notifications";
pub const UI_PATH: &str = "/org/gofroshka/Notifd";
pub const UI_INTERFACE: &str = "org.gofroshka.Notifd1";

/// Register the D-Bus service, claim the well-known name and start forwarding
/// domain events to D-Bus signals. Claiming the name also signals readiness to
/// systemd (`Type=dbus`).
pub async fn serve(state: Arc<State>, config: Config) -> Result<Connection> {
    let connection = zbus::connection::Builder::session()?
        .name(INTERFACE)?
        .serve_at(
            PATH,
            interface::Notifications {
                state: state.clone(),
                config,
            },
        )?
        .serve_at(
            UI_PATH,
            ui::Ui {
                state: state.clone(),
            },
        )?
        .build()
        .await?;

    signals::spawn(connection.clone(), state.subscribe());
    Ok(connection)
}
