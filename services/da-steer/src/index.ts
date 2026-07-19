// da-steer — the durable park behind bin/steer (ADR-0029) plus the DaRun
// run mirror. One workflow instance per steer-request (key minted by
// bin/steer: <run-id>--<stage>--<content-fingerprint>, sanitized): `run`
// parks on an awakeable until the operator answers; shared handlers expose
// the request and the awakeable id so the file<->Restate bridge (bin/steer)
// and the operator (curl / the Restate UI) can reach it.
//
// The FILE (stages/<NN>/output/STEER-REQUEST.md) stays canonical — this
// service holds no truth of its own, only the durable wait and the answer
// in transit. Behavior lives in logic.ts behind a narrow seam; this file is
// only the SDK binding.

import * as http2 from "node:http2";
import * as restate from "@restatedev/restate-sdk";
import {
  type DerivedState,
  type RunArtifact,
  type RunSnapshot,
  type SteerRequest,
  type Steering,
  getSnapshot,
  getState,
  parkSteer,
  recordSnapshot,
  steerAwakeableId,
  steerRequest,
} from "./logic.js";

const daRun = restate.object({
  name: "DaRun",
  handlers: {
    recordSnapshot: async (
      ctx: restate.ObjectContext,
      snapshot: { state: DerivedState; files?: RunArtifact[] }
    ): Promise<void> => recordSnapshot(ctx, snapshot),

    getState: restate.handlers.object.shared(
      async (ctx: restate.ObjectSharedContext): Promise<DerivedState | null> => getState(ctx)
    ),

    getSnapshot: restate.handlers.object.shared(
      async (ctx: restate.ObjectSharedContext): Promise<RunSnapshot> => getSnapshot(ctx)
    ),
  },
});

const daSteer = restate.workflow({
  name: "DaSteer",
  handlers: {
    run: async (ctx: restate.WorkflowContext, req: SteerRequest): Promise<Steering> =>
      parkSteer(ctx, { awakeable: <T>() => ctx.awakeable<T>() }, req),

    // Shared handlers: callable while `run` is parked (and after), without
    // joining the queue.
    awakeableId: async (ctx: restate.WorkflowSharedContext): Promise<string | null> =>
      steerAwakeableId(ctx),

    request: async (ctx: restate.WorkflowSharedContext): Promise<SteerRequest | null> =>
      steerRequest(ctx),
  },
});

// Bind address from the environment: DA_STEER_LISTEN as "host:port" (or a
// bare port). Default unchanged — all interfaces, :9080. The systemd unit
// pins the tailnet address so the listener is not reachable off-tailnet
// even before Tailscale's own filtering. The SDK's own listen() cannot
// bind a host, so the HTTP/2 server is instantiated here.
const listen: string = process.env.DA_STEER_LISTEN ?? "9080";
const separator: number = listen.lastIndexOf(":");
const host: string | undefined = separator > 0 ? listen.slice(0, separator) : undefined;
const port: number = Number.parseInt(separator > 0 ? listen.slice(separator + 1) : listen, 10);
if (Number.isNaN(port)) {
  throw new Error(`DA_STEER_LISTEN ${JSON.stringify(listen)} — expected "port" or "host:port"`);
}
const server: http2.Http2Server = http2.createServer(
  restate.endpoint().bind(daSteer).bind(daRun).http2Handler()
);
server.listen(port, host);
