mod config;
mod dbus;
mod hints;
mod image_cache;
mod model;
mod socket;
mod sound;
mod state;

use std::sync::Arc;

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = config::Config::from_env();
    let initial_dnd = std::fs::read_to_string(&config.dnd_file)
        .map(|value| value.trim() == "1")
        .unwrap_or(false);
    let state = Arc::new(state::State::new(
        initial_dnd,
        Some(config.dnd_file.clone()),
    ));

    // Claiming `org.freedesktop.Notifications` also reports readiness to
    // systemd (the unit is `Type=dbus`).
    let _connection = dbus::serve(state.clone(), config.clone()).await?;
    socket::spawn(config.socket_path.clone(), state.clone()).await?;

    log::info!("notifd ready");
    tokio::signal::ctrl_c().await?;
    Ok(())
}
