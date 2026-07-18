# da-steer — durable steer park + run-state mirror (ADR-0029)

One Restate endpoint, two services. **DaSteer**: a workflow per steer-request, parked on an
awakeable until the operator answers — `bin/steer park` starts it, bridges the file answer
into the awakeable (and vice versa), and unblocks the arm. **DaRun**: a virtual object per
run mirroring the derived run state published by `bin/state notify`. The FILES stay
canonical for both; this endpoint holds only the durable wait and a read model.

## Run + register (homelab)

```sh
cd services/da-steer
npm install
npm run build            # the systemd unit runs dist/, not tsx
npm run dev              # or: run directly; listens on :9080

# register with the homelab Restate via its admin API (no restate CLI on this host):
CA=~/.local/share/caddy/pki/authorities/local/root.crt
curl -sS --cacert $CA -X POST https://restate.homelab/deployments \
     -H 'content-type: application/json' \
     -d '{"uri":"http://fedora.mist-walleye.ts.net:9080/","force":true}'
curl -sS --cacert $CA https://restate.homelab/services   # expect DaSteer and DaRun
```

In production it runs as the `da-steer` systemd user unit — see `infra/systemd/README.md`
(install, restart-vs-re-register rules).

## DaRun — the run-state mirror

The endpoint also binds `DaRun`, a virtual object keyed by run-id: `recordState` stores the
derived run state (wire payload from `da-state`), `getState` reads it. Non-authoritative and
last-writer-wins — the run dir stays canonical; this is a read model for anything on the
tailnet that wants run status without filesystem access.

```sh
DA_STEER_INGRESS=https://restate-ingress.homelab bin/state notify --run <RUNDIR>   # publish
curl -sS --cacert $CA -X POST https://restate-ingress.homelab/DaRun/<run-id>/getState \
     -H 'content-type: application/json' -d '{}'                                   # read
```

The payload is the versioned wire shape (`crates/wire`) — read it tolerantly. A deployment
that adds a service (as the DaRun build did) must be re-registered before Restate routes to
it; see `infra/systemd/README.md` for the admin-API call.

## Use

`algorithm/bin/steer` uses it automatically once `DA_STEER_INGRESS` is set:

```sh
export DA_STEER_INGRESS=https://restate-ingress.homelab
# RESTATE_CA defaults to ~/.local/share/caddy/pki/authorities/local/root.crt
bb algorithm/bin/steer park --run <RUNDIR>   # a steer now parks durably instead of exiting 3
```

Answer a parked steer any of three ways (all converge — bin/steer bridges both directions):

```sh
# 1. edit '## Answer' in stages/<NN>/output/STEER-REQUEST.md          (the ICM way)
# 2. bin/steer resolve --run <RUNDIR> --stage <NN> --answer "..."     (CLI)
# 3. resolve the awakeable directly                                   (remote / UI)
KEY=<run-id>--<stage>   # sanitized: [^A-Za-z0-9_-] -> '-'
AID=$(curl -sS --cacert $CA -X POST https://restate-ingress.homelab/DaSteer/$KEY/awakeableId \
      -H 'content-type: application/json' -d '{}')
curl -sS --cacert $CA -X POST https://restate-ingress.homelab/restate/awakeables/$AID/resolve \
     -H 'content-type: application/json' -d '{"answer": "use 9080"}'
```

Observe parked steers in the UI at https://restate.homelab, or via the admin query API
(`POST https://restate.homelab/query`, `accept: application/json`):
`SELECT id, target, status FROM sys_invocation WHERE status != 'completed'` — a parked
steer shows `run` suspended on its awakeable.

## Waiting without blocking a session

From an interactive Claude Code session, don't foreground `bin/steer park` — run it with
`run_in_background`, or point a Monitor at the workflow output endpoint
(`/restate/workflow/DaSteer/<key>/output` returns 200 once answered) or at
`bin/steer check --run <RUNDIR>` (exit 0 = all answered) and get woken when the answer lands.

## Gotchas (from the homelab Restate guide)

- State ops (`ctx.set`) are journaled; the awakeable is created deterministically — safe on
  replay. Side effects beyond that belong in `ctx.run`.
- The awakeable id is the capability — tailnet-only, no further auth. Don't paste it anywhere
  public.
- A green `/version` proves nothing; trust the invocation list (UI or the query above).
