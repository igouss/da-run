# infra/systemd — version-tracked user units

The unit files live here in the bundle; `~/.config/systemd/user/` holds symlinks to them,
so updating the bundle updates the installed unit (followed by
`systemctl --user daemon-reload`).

## Install

```sh
# adjust the path to wherever the bundle lives (~/code/da-run2, ~/.claude/skills/da-run, …)
ln -sf ~/code/da-run2/infra/systemd/da-steer.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now da-steer.service
```

The unit's `WorkingDirectory` must agree with the symlink target's checkout — it points at
`%h/code/da-run2/services/da-steer`; adjust both together if the bundle lives elsewhere.

## Units

| Unit | What |
|------|------|
| `da-steer.service` | one Restate endpoint serving **DaSteer** (durable steer park behind `bin/steer park`, ADR-0029) and **DaRun** (the run-state mirror fed by `bin/state notify`) — registered with the homelab Restate as `http://fedora.mist-walleye.ts.net:9080` |

## Bind address

The service reads `DA_STEER_LISTEN` (`host:port`, or a bare port; default `9080` on all
interfaces). The unit pins it to `fedora.mist-walleye.ts.net:9080` — the tailnet address —
so the listener never faces non-tailnet interfaces; the address it binds MUST be the one
registered with Restate (below). Binding to the tailnet name needs tailscaled up at start;
the unit orders after `network-online.target`.

**NEXT-STEPS (operator):** the live service still runs with the pre-bind-address build.
Applying this change is: `npm run build` in `services/da-steer/`, `systemctl --user
daemon-reload` (the unit gained an `Environment=` line), `systemctl --user restart
da-steer`. Same URI as registered, so no re-registration — but confirm nothing is parked
before restarting (query below).

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

`force:true` overwrites the deployment record at that URI — routine for this endpoint, which
only ever grows (new services/handlers, never removals in-flight invocations need). Before
forcing, confirm nothing is parked:
`SELECT id, target, status FROM sys_invocation WHERE status != 'completed'` against
`https://restate.homelab/query` (`accept: application/json`).
