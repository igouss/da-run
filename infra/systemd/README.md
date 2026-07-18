# infra/systemd — version-tracked user units

The unit files live here in the bundle; `~/.config/systemd/user/` holds symlinks to them,
so updating the bundle updates the installed unit (followed by
`systemctl --user daemon-reload`).

## Install

```sh
# adjust the path to wherever the bundle lives (~/code/da-run, ~/.claude/skills/da-run, …)
ln -sf ~/code/da-run/infra/systemd/da-steer.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now da-steer.service
```

The unit's `WorkingDirectory` must agree with the symlink target's checkout — it points at
`%h/code/da-run/services/da-steer`; adjust both together if the bundle lives elsewhere.

## Units

| Unit | What |
|------|------|
| `da-steer.service` | one Restate endpoint on :9080 serving **DaSteer** (durable steer park behind `bin/steer park`, ADR-0029) and **DaRun** (the run-state mirror fed by `bin/state notify`) — registered with the homelab Restate as `http://fedora.mist-walleye.ts.net:9080` |

After changing the service source: `npm run build` in `services/da-steer/`, then
`systemctl --user restart da-steer`. A plain restart needs no re-registration — parked
invocations back off and replay on reconnect. Re-register when the host/port changes **or
the endpoint gains a service/handler** (discovery happens only at registration):

```sh
# no restate CLI on this host — the admin API (https://restate.homelab) does it:
curl -sS --cacert ~/.local/share/caddy/pki/authorities/local/root.crt \
     -X POST https://restate.homelab/deployments \
     -H 'content-type: application/json' \
     -d '{"uri":"http://fedora.mist-walleye.ts.net:9080/","force":true}'
```

`force:true` is safe here only because the endpoint stays backward-compatible (it adds
services, never removes handlers in-flight invocations need). Check for parked work first:
`SELECT id, target, status FROM sys_invocation WHERE status != 'completed'` against
`https://restate.homelab/query`.
