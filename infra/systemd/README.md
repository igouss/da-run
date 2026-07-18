# infra/systemd — version-tracked user units

The unit files live here in the bundle; `~/.config/systemd/user/` holds symlinks to them,
so updating the bundle updates the installed unit (followed by
`systemctl --user daemon-reload`).

## Install

```sh
# adjust the path if the bundle is installed somewhere other than ~/.claude/skills/da-run
ln -sf ~/.claude/skills/da-run/infra/systemd/da-steer.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now da-steer.service
```

## Units

| Unit | What |
|------|------|
| `da-steer.service` | the Restate workflow service behind `bin/steer park` (ADR-0029) — listens on :9080, registered with the homelab Restate as `http://fedora.mist-walleye.ts.net:9080` |

After changing the service source: `npm run build` in `services/da-steer/`, then
`systemctl --user restart da-steer`. Re-register with Restate only if the host/port
changes; restarts need nothing — parked invocations back off and replay on reconnect.
