# notifd

A small notification daemon for Linux desktops, written in Rust. It owns the
D-Bus name `org.freedesktop.Notifications`, implements the Desktop Notifications
Specification (plus sound, a real DND `Inhibited` property and persistence), and
exposes a typed D-Bus interface for a separate UI process.

It is designed to pair with a UI-only shell (this one is used with
[Quickshell](https://quickshell.org/)). Any D-Bus client can consume the UI
interface; the standard notification API remains unchanged.

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
- `org.gofroshka.Notifd1`: typed snapshot, change signal and UI actions on the
  same session bus as the notification API.
- Ready for systemd (`Type=dbus`).

## Architecture

```
app ──D-Bus org.freedesktop.Notifications──► notifd ◄──D-Bus org.gofroshka.Notifd1──► UI (Quickshell)
```

The daemon owns the standard name `org.freedesktop.Notifications`; the UI
queries its private interface and subscribes to changes over D-Bus.

## Building

With Cargo:

```sh
cargo build --release
```

Or with Nix:

```sh
nix build
```

For development, the repo ships a flake dev shell wired up for
[direnv](https://direnv.net/). After `direnv allow` the shell (cargo,
rust-analyzer, libcanberra, pulseaudio) loads automatically.

## Running

```sh
NOTIFD_STANDARD_SOUND=/path/to/default.wav notifd
```

### Configuration

All configuration is via environment variables, so the systemd unit stays
declarative:

| Variable | Default | Meaning |
|---|---|---|
| `NOTIFD_STANDARD_SOUND` | `message-new-instant` | Default sound (XDG sound name or absolute file path). Empty string disables the fallback. |

DND state is stored in `$XDG_STATE_HOME/notifd/dnd` and restored on start.

## UI D-Bus interface

Service `org.freedesktop.Notifications`, object `/org/gofroshka/Notifd`,
interface `org.gofroshka.Notifd1`:

- `GetSnapshot() → (b, a(ussssssbba(ss)))`: the DND flag and notifications.
  Each notification contains id, app name, app icon, summary, body, urgency
  (`low`/`normal`/`critical`), image path (empty if absent), inline-reply flag,
  resident flag and `(action id, label)` pairs. Fetch this on startup and after
  change signals; the snapshot is atomic and also recovers missed events.
- `Changed(kind: s, id: u)`: emitted after a state change. `kind` is `added`,
  `updated`, `removed` or `dnd`; `id` is 0 for DND changes. Clients should
  refetch the snapshot rather than relying on event payloads.
- `SetDnd(b)`, `Dismiss(u)`, `Invoke(u, s)`, `Reply(u, s)`, `Clear()`:
  actions from the UI. Dismiss/clear keep their
  user-dismissed `NotificationClosed` reason on the standard interface.

For example:

```sh
busctl --user call org.freedesktop.Notifications /org/gofroshka/Notifd \
  org.gofroshka.Notifd1 GetSnapshot
busctl --user call org.freedesktop.Notifications /org/gofroshka/Notifd \
  org.gofroshka.Notifd1 SetDnd b true
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
