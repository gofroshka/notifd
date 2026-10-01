use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::unix::OwnedWriteHalf;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::broadcast::error::RecvError;

use super::protocol::{ClientCommand, ServerEvent};
use crate::model::CloseReason;
use crate::state::State;

/// Bind the UI socket and serve clients until the process exits.
pub async fn spawn(path: PathBuf, state: Arc<State>) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let _ = std::fs::remove_file(&path);

    let listener = UnixListener::bind(&path)?;
    log::info!("UI socket listening on {}", path.display());

    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let state = state.clone();
                    tokio::spawn(async move {
                        if let Err(error) = handle(stream, state).await {
                            log::debug!("UI client disconnected: {error}");
                        }
                    });
                }
                Err(error) => log::warn!("accept failed: {error}"),
            }
        }
    });
    Ok(())
}

async fn handle(stream: UnixStream, state: Arc<State>) -> Result<()> {
    let (read_half, mut write_half) = stream.into_split();
    let mut events = state.subscribe();

    let (dnd, notifications) = state.snapshot();
    write_event(&mut write_half, &ServerEvent::Snapshot { dnd, notifications }).await?;

    let mut lines = BufReader::new(read_half).lines();
    loop {
        tokio::select! {
            line = lines.next_line() => match line? {
                Some(line) if !line.trim().is_empty() => apply(&state, &line),
                Some(_) => {}
                None => break,
            },
            event = events.recv() => match event {
                Ok(event) => {
                    if let Some(server_event) = ServerEvent::from_event(event) {
                        write_event(&mut write_half, &server_event).await?;
                    }
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            },
        }
    }

    Ok(())
}

fn apply(state: &State, line: &str) {
    match serde_json::from_str::<ClientCommand>(line) {
        Ok(ClientCommand::SetDnd { value }) => state.set_dnd(value),
        Ok(ClientCommand::Dismiss { id }) => {
            state.close(id, CloseReason::Dismissed);
        }
        Ok(ClientCommand::Invoke { id, key }) => state.invoke(id, key),
        Ok(ClientCommand::Reply { id, text }) => state.reply(id, text),
        Ok(ClientCommand::Clear) => state.clear(CloseReason::Dismissed),
        Err(error) => log::warn!("ignoring malformed UI command: {error}"),
    }
}

async fn write_event(writer: &mut OwnedWriteHalf, event: &ServerEvent) -> Result<()> {
    let mut line = serde_json::to_string(event)?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;
    Ok(())
}
