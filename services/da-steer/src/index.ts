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

restate.endpoint().bind(daSteer).bind(daRun).listen(9080);
