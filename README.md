# notifd

A small notification daemon for Linux desktops, written in Rust. It owns the
D-Bus name `org.freedesktop.Notifications`, implements the Desktop Notifications
Specification (plus sound, a real DND `Inhibited` property and persistence), and
mirrors its state to a separate UI process over a Unix socket.

It is designed to pair with a UI-only shell (this one is used with
[Quickshell](https://quickshell.org/)), but the socket protocol is plain
JSON-lines, so any frontend can consume it.

## Why split the daemon from the UI?

Most shells embed their notification server. Separating the backend from the
presentation gives you:

- **One source of truth for Do Not Disturb.** `notifd` exposes the standard
  `Inhibited` property, so applications that honour it — Telegram Desktop among
  them — stop playing their own notification sound while DND is active.
- **A single, refined default sound.** Apps that ask for the default get the
  configured system sound; apps that bring their own `sound-file` / `sound-name`
  keep their own.
- **A UI that survives reloads.** Notifications live in the daemon, so reloading
  the shell no longer drops them.

## Features

- Full `org.freedesktop.Notifications` implementation: `Notify`,
  `CloseNotification`, `GetCapabilities`, `GetServerInformation` and the
  `NotificationClosed` / `ActionInvoked` / `NotificationReplied` signals.
- `Inhibited` property and `Inhibit` / `UnInhibit` methods.
- Sound policy: honours `sound-file`, `sound-name` and `suppress-sound`; falls
  back to a configurable standard sound; silent under DND. Playback goes through
  libcanberra (`media.role=event`) with a `paplay` fallback.
- `image-data` / `image-path` decoding into cached PNGs.
- `replaces_id`, `expire_timeout` and persistence (`resident`) handling.
- DND state persisted across restarts.
- A JSON-lines Unix socket for the UI: an initial snapshot, incremental events,
  and user commands back.
- Ready for systemd (`Type=dbus`).

## Architecture

```
app ──D-Bus org.freedesktop.Notifications──► notifd ──Unix socket (JSON lines)──► UI (Quickshell)
```

The daemon is the only owner of the D-Bus name. The UI never talks to the bus;
it mirrors daemon state and forwards user actions (dismiss, invoke, reply, clear,
toggle DND).

## Building

With Cargo:

```sh
cargo build --release
```

Or with Nix:

```sh
nix build
```

## Running

```sh
NOTIFD_STANDARD_SOUND=/path/to/default.wav notifd
```

### Configuration

All configuration is via environment variables, so the systemd unit stays
declarative:

| Variable | Default | Meaning |
|---|---|---|
| `NOTIFD_SOCKET` | `$XDG_RUNTIME_DIR/notifd.sock` | Unix socket for the UI |
| `NOTIFD_STANDARD_SOUND` | `message-new-instant` | Default sound (XDG sound name or absolute file path). Empty string disables the fallback. |

DND state is stored in `$XDG_STATE_HOME/notifd/dnd` and restored on start.

## UI socket protocol

Each connection receives a snapshot, then a stream of events (one JSON object
per line). The UI sends commands back on the same socket.

Server → UI:

```json
{"type":"snapshot","dnd":false,"notifications":[...]}
{"type":"added","notification":{...}}
{"type":"updated","notification":{...}}
{"type":"removed","id":1,"reason":2}
{"type":"dnd","value":true}
```

UI → server:

```json
{"type":"dismiss","id":1}
{"type":"invoke","id":1,"key":"default"}
{"type":"reply","id":1,"text":"hello"}
{"type":"clear"}
{"type":"set_dnd","value":true}
```

## systemd

```ini
[Unit]
Description=Notification daemon
After=graphical-session.target
PartOf=graphical-session.target

[Service]
Type=dbus
BusName=org.freedesktop.Notifications
ExecStart=%h/.nix-profile/bin/notifd
Environment=NOTIFD_STANDARD_SOUND=/path/to/default.wav
Restart=on-failure
RestartSec=2

[Install]
WantedBy=graphical-session.target
```

`Type=dbus` makes systemd consider the unit started only once
`org.freedesktop.Notifications` is claimed, which lets a UI unit order itself
after it with `After=notifd.service`.

## License

MIT — see [LICENSE](LICENSE).
