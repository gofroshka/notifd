mod interface;
mod signals;

use std::sync::Arc;

use anyhow::Result;
use zbus::Connection;

use crate::config::Config;
use crate::state::State;

pub const PATH: &str = "/org/freedesktop/Notifications";
pub const INTERFACE: &str = "org.freedesktop.Notifications";

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
        .build()
        .await?;

    signals::spawn(connection.clone(), state.subscribe());
    Ok(connection)
}
