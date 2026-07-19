// da-steer — the durable park behind bin/steer (ADR-0029). One workflow instance per
// steer-request (key = <run-id>--<stage>, sanitized): `run` parks on an awakeable until the
// operator answers; shared handlers expose the request and the awakeable id so the file<->
// Restate bridge (bin/steer) and the operator (curl / the Restate UI) can reach it.
//
// The FILE (stages/<NN>/output/STEER-REQUEST.md) stays canonical — this service holds no
// truth of its own, only the durable wait and the answer in transit.

import * as restate from "@restatedev/restate-sdk";

type SteerRequest = {
  runId: string;
  stage: string;
  question: string;
  options: string;
  file: string;
};

type Steering = {
  answer: string;
  notes?: string;
};

// The non-authoritative run mirror: `da-state notify` publishes the derived
// state (wire payload v1) after status/check; anything on the tailnet can read
// it without touching the run dir. Last-writer-wins — the filesystem stays
// canonical, so a stale mirror is a display problem, never a truth problem.
type DerivedState = {
  v: number;
  run_id: string;
  state: string;
  phase: string;
  parked: string[];
  anomalies: unknown[];
};

// One run artifact: a run-dir-relative path and its UTF-8 content. Artifacts
// are the run's durable ephemera — run.edn, flow.ron, spec.md, and every
// stage's output/ files — pushed by `da-state notify` after each change, so
// the run can be restored on another host with `da-state restore`. The
// worktree is never mirrored: code lives in the target project's git.
type RunArtifact = {
  path: string;
  content: string;
};

type RunArtifacts = {
  files: RunArtifact[];
};

// The restore payload: the last recorded state plus the artifact set.
type RunSnapshot = {
  state: DerivedState | null;
  files: RunArtifact[];
};

const daRun = restate.object({
  name: "DaRun",
  handlers: {
    recordState: async (
      ctx: restate.ObjectContext,
      derived: DerivedState
    ): Promise<void> => {
      ctx.set("state", derived);
    },

    // Full-replace semantics: the run dir is canonical, the mirror follows.
    // Each push carries the complete artifact set, so a file deleted on disk
    // (an operator retracting an output) also disappears from the mirror.
    recordArtifacts: async (
      ctx: restate.ObjectContext,
      batch: RunArtifacts
    ): Promise<void> => {
      ctx.set("artifacts", batch.files ?? []);
    },

    getState: restate.handlers.object.shared(
      async (ctx: restate.ObjectSharedContext): Promise<DerivedState | null> =>
        (await ctx.get<DerivedState>("state")) ?? null
    ),

    getSnapshot: restate.handlers.object.shared(
      async (ctx: restate.ObjectSharedContext): Promise<RunSnapshot> => ({
        state: (await ctx.get<DerivedState>("state")) ?? null,
        files: (await ctx.get<RunArtifact[]>("artifacts")) ?? [],
      })
    ),
  },
});

const daSteer = restate.workflow({
  name: "DaSteer",
  handlers: {
    run: async (ctx: restate.WorkflowContext, req: SteerRequest): Promise<Steering> => {
      ctx.set("request", req);
      // Park durably until someone resolves the awakeable — bin/steer (bridging a file
      // answer), a raw curl, or the Restate UI. Survives crashes and redeploys; days if
      // needed. The id is the capability: it is exposed via the shared handler below and
      // must travel no further than the operator's own tailnet.
      const awakeable = ctx.awakeable<Steering>();
      ctx.set("awakeableId", awakeable.id);
      const steering: Steering = await awakeable.promise;
      ctx.set("answer", steering);
      return steering;
    },

    // Shared handlers: callable while `run` is parked (and after), without joining the queue.
    awakeableId: async (ctx: restate.WorkflowSharedContext): Promise<string | null> =>
      (await ctx.get<string>("awakeableId")) ?? null,

    request: async (ctx: restate.WorkflowSharedContext): Promise<SteerRequest | null> =>
      (await ctx.get<SteerRequest>("request")) ?? null,
  },
});

restate.endpoint().bind(daSteer).bind(daRun).listen(9080);
