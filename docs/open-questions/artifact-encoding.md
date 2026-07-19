# Open question: artifact contents are lossy for non-UTF-8 bytes

**Status:** open — raised 2026-07-18. Severity: low today, latent in the durability layer.
**Area:** `crates/adapter-fs/src/run_artifacts.rs:103`.

---

## What happens

Artifacts are `String`, and collection converts bytes with:

```rust
content: String::from_utf8_lossy(&bytes).into_owned(),   // :103
```

Any byte sequence that is not valid UTF-8 is replaced with U+FFFD and round-trips as corrupted.
No error, no warning — the artifact simply comes back different from how it went in.

## Why it is low severity today

Everything currently collected is text authored by agents or by git: `run.edn`, `flow.ron`,
`spec.md`, markdown stage outputs, and `worktree.patch`. The patch is the interesting one, and
it is safe *by construction*: `seal` uses `git diff --binary`, and git ASCII-armours binary
hunks (base85 inside a text patch), so the output is valid UTF-8 even when the change touches
binary files. That was a deliberate choice — see the seal comment in `engine/bin/run` — and it
is the reason this issue is not already biting.

## When it would bite

- A stage writes an output file in a non-UTF-8 encoding (a tool dumping latin-1 logs, a test
  fixture with raw bytes).
- A future artifact is added to `ROOT_FILES` that is genuinely binary and not git-armoured.
- A spec or design doc is pasted with an encoding mismatch.

In each case the corruption is silent and lands in the *durability* layer — the part whose
whole job is to return exactly what it was given.

## Effect on you

You would not see it at publish time. You would see it after a restore, as a file with U+FFFD
replacement characters where content used to be — most confusingly if it is `worktree.patch`,
because a corrupted patch fails to apply and the restore reports a patch that "did not apply
cleanly" with no hint that the mirror mangled it rather than the patch being wrong.

## Options

**A. Leave it.** Everything collected today is UTF-8 by construction.
*Cost:* a silent-corruption path stays open in the layer that must not have one, and the
guarantee depends on a property (git armouring) that a future contributor may not know about.

**B. Error on non-UTF-8 instead of lossy-converting.** Replace `from_utf8_lossy` with
`String::from_utf8` and return a `SnapshotError` naming the file.
*Cost:* one-line change plus a test. Turns silent corruption into a loud, actionable failure.
A stage that legitimately emits binary output would then fail to publish — which is arguably
correct, since today it publishes garbage instead.

**C. Make artifact content an enum (`Text(String)` / `Binary(Vec<u8>)`), base64 on the wire.**
The complete fix.
*Cost:* ripples through ports, wire, the restate service, and any stored mirror payload.
Considered during the durability work and rejected precisely because `--binary` patches made it
unnecessary; it would be re-justified only by a real binary artifact.

## Recommendation on record

**B.** It is nearly free and converts an invisible failure into an obvious one, which is the
right trade in a durability path. C stays on the shelf until something genuinely binary needs
mirroring; if that day comes, B's error is exactly the signal that will tell us.

## Anchors

- `crates/adapter-fs/src/run_artifacts.rs:103` — the lossy conversion.
- `engine/bin/run` (`seal!`) — why the patch is UTF-8-safe today.
