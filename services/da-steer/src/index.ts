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

restate.endpoint().bind(daSteer).listen(9080);
