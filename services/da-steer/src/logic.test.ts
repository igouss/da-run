// The service behavior over the narrow seam — no Restate needed. What these
// prove: recordSnapshot's full-replace + one-invocation atomicity contract,
// getSnapshot's shape for da-state restore, and the park lifecycle bin/steer
// bridges against (request exposed while parked, answer recorded after).

import { describe, expect, it } from "vitest";
import {
  type DerivedState,
  type Parking,
  type StateStore,
  type Steering,
  getSnapshot,
  getState,
  parkSteer,
  recordSnapshot,
  steerAwakeableId,
  steerRequest,
} from "./logic.js";

class FakeStore implements StateStore {
  private entries: Map<string, unknown> = new Map();

  async get<T>(key: string): Promise<T | null> {
    return (this.entries.get(key) as T | undefined) ?? null;
  }

  set<T>(key: string, value: T): void {
    this.entries.set(key, value);
  }
}

function derived(state: string): DerivedState {
  return { v: 1, run_id: "42-proj-beef", state, phase: "steady-state", parked: [], anomalies: [] };
}

describe("DaRun mirror", () => {
  it("an empty mirror snapshots to null state and zero files", async () => {
    const store: FakeStore = new FakeStore();
    expect(await getState(store)).toBeNull();
    expect(await getSnapshot(store)).toEqual({ state: null, files: [] });
  });

  it("one recordSnapshot lands state and files together", async () => {
    const store: FakeStore = new FakeStore();
    await recordSnapshot(store, {
      state: derived("gated-green"),
      files: [{ path: "run.edn", content: "{}" }],
    });
    const snapshot = await getSnapshot(store);
    expect(snapshot.state?.state).toBe("gated-green");
    expect(snapshot.files).toHaveLength(1);
  });

  it("many pushes full-replace: a file retracted on disk vanishes from the mirror", async () => {
    const store: FakeStore = new FakeStore();
    await recordSnapshot(store, {
      state: derived("designed"),
      files: [
        { path: "run.edn", content: "{}" },
        { path: "stages/01-plan/output/plan.md", content: "# plan" },
      ],
    });
    await recordSnapshot(store, {
      state: derived("gated-red"),
      files: [{ path: "run.edn", content: "{}" }],
    });
    const snapshot = await getSnapshot(store);
    expect(snapshot.state?.state).toBe("gated-red");
    expect(snapshot.files.map((file) => file.path)).toEqual(["run.edn"]);
  });

  it("absent files field replaces with an empty set, not a stale one", async () => {
    const store: FakeStore = new FakeStore();
    await recordSnapshot(store, {
      state: derived("designed"),
      files: [{ path: "spec.md", content: "s" }],
    });
    await recordSnapshot(store, { state: derived("designed") });
    expect((await getSnapshot(store)).files).toEqual([]);
  });
});

describe("DaSteer park", () => {
  function parkingWith(promise: Promise<Steering>): Parking {
    return { awakeable: <T>() => ({ id: "aw_1", promise: promise as Promise<T> }) };
  }

  it("exposes the request and awakeable id while parked, records the answer after", async () => {
    const store: FakeStore = new FakeStore();
    let release: (steering: Steering) => void = () => {};
    const gate: Promise<Steering> = new Promise((resolve) => {
      release = resolve;
    });
    const request = {
      runId: "42-proj-beef",
      stage: "02-build",
      question: "Which port?",
      options: "- A: 9080",
      file: "/run/stages/02-build/output/STEER-REQUEST.md",
    };

    const parked: Promise<Steering> = parkSteer(store, parkingWith(gate), request);
    // While parked: bin/steer reads these to bridge and to resolve.
    expect(await steerRequest(store)).toEqual(request);
    expect(await steerAwakeableId(store)).toBe("aw_1");
    expect(await store.get("answer")).toBeNull();

    release({ answer: "use 9080" });
    expect(await parked).toEqual({ answer: "use 9080" });
    expect(await store.get<Steering>("answer")).toEqual({ answer: "use 9080" });
  });

  it("a fresh instance exposes neither request nor awakeable id", async () => {
    const store: FakeStore = new FakeStore();
    expect(await steerRequest(store)).toBeNull();
    expect(await steerAwakeableId(store)).toBeNull();
  });
});
