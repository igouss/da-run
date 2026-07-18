# da-steer — durable steer-request park (ADR-0029)

A minimal Restate workflow service: one instance per steer-request, parked on an awakeable
until the operator answers. `bin/steer park` starts it, bridges the file answer into the
awakeable (and vice versa), and unblocks the arm. The steer FILE stays canonical; this
service only holds the durable wait.

## Run + register (homelab)

```sh
cd services/da-steer
npm install
npm run typecheck        # or: npm run build
npm run dev              # listens on :9080

# register with the homelab Restate (service running on fedora, on the tailnet):
restate deployments register http://fedora.mist-walleye.ts.net:9080
restate services list    # expect DaSteer and DaRun
```

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

After deploying a build that first adds `DaRun`, re-register the deployment (new services are
only discovered at registration): `restate deployments register http://fedora.mist-walleye.ts.net:9080 --force`.

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

Observe parked steers: `restate invocations list` (or the UI at https://restate.homelab) —
a parked steer shows `run` suspended on its awakeable.

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
- A green `/version` proves nothing; trust `restate invocations list`.
