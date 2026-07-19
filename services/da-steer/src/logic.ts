// The service's behavior over a narrow context seam, so it is testable
// without a running Restate. index.ts binds these to the SDK; the seam
// carries exactly the journaled operations the handlers use (get/set/
// awakeable) and nothing else — logic here must stay deterministic.

export type SteerRequest = {
  runId: string;
  stage: string;
  question: string;
  options: string;
  file: string;
};

export type Steering = {
  answer: string;
  notes?: string;
};

// The non-authoritative run mirror payload: `da-state notify` publishes the
// derived state (wire payload v1) and the full artifact set in ONE call, so
// the mirror can never advertise a state its artifacts do not support.
export type DerivedState = {
  v: number;
  run_id: string;
  state: string;
  phase: string;
  parked: string[];
  anomalies: unknown[];
};

export type RunArtifact = {
  path: string;
  content: string;
};

export type RunSnapshot = {
  state: DerivedState | null;
  files: RunArtifact[];
};

// The journaled key-value surface. Shared handlers only read, so the seam
// splits: exclusive handlers get the store, shared handlers the reader.
export interface StateReader {
  get<T>(key: string): Promise<T | null>;
}

export interface StateStore extends StateReader {
  set<T>(key: string, value: T): void;
}

// The durable-wait surface DaSteer parks on.
export interface Parking {
  awakeable<T>(): { id: string; promise: Promise<T> };
}

// ---- DaRun (the mirror) ---------------------------------------------------

// Full-replace semantics: the run dir is canonical, the mirror follows. A
// file deleted on disk (an operator retracting an output) also disappears
// from the mirror. State and files land in one handler invocation — Restate
// commits both or neither.
export async function recordSnapshot(
  store: StateStore,
  snapshot: { state: DerivedState; files?: RunArtifact[] }
): Promise<void> {
  store.set("state", snapshot.state);
  store.set("artifacts", snapshot.files ?? []);
}

export async function getState(store: StateReader): Promise<DerivedState | null> {
  return (await store.get<DerivedState>("state")) ?? null;
}

export async function getSnapshot(store: StateReader): Promise<RunSnapshot> {
  return {
    state: (await store.get<DerivedState>("state")) ?? null,
    files: (await store.get<RunArtifact[]>("artifacts")) ?? [],
  };
}

// ---- DaSteer (the durable park) --------------------------------------------

// Park durably until someone resolves the awakeable — bin/steer (bridging a
// file answer), a raw curl, or the Restate UI. Survives crashes and
// redeploys; days if needed. The workflow key is minted by bin/steer as
// <run-id>--<stage>--<content-fingerprint>: content in the key is what makes
// a NEW question a NEW workflow, so a completed park can never answer a
// later, different ask (da-run's stale-answer hole).
export async function parkSteer(
  store: StateStore,
  parking: Parking,
  req: SteerRequest
): Promise<Steering> {
  store.set("request", req);
  const awakeable: { id: string; promise: Promise<Steering> } = parking.awakeable<Steering>();
  store.set("awakeableId", awakeable.id);
  const steering: Steering = await awakeable.promise;
  store.set("answer", steering);
  return steering;
}

export async function steerAwakeableId(store: StateReader): Promise<string | null> {
  return (await store.get<string>("awakeableId")) ?? null;
}

export async function steerRequest(store: StateReader): Promise<SteerRequest | null> {
  return (await store.get<SteerRequest>("request")) ?? null;
}
