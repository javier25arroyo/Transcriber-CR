# ADR 0001: Isolated STT sidecar process

- **Status:** Accepted
- **Date:** 2026-07-26
- **Source:** [Product spec §6](../superpowers/specs/2026-07-25-voice-type-dictation-design.md#6-arquitectura)

## Context

`whisper.cpp` (via `whisper-rs`) is native C++ code running with CUDA. A
segfault or a VRAM out-of-memory condition in that code is not recoverable
from within the same process — it takes the whole application down with it,
including the UI. Voice Type needs to run unattended as a background service;
a crash in the speech engine cannot be allowed to kill the tray icon and the
hotkey listener along with it.

## Decision

The STT engine runs in a separate child process (the "sidecar"), spawned and
supervised by the main Tauri process. Communication happens over
stdin/stdout using a JSON-line protocol plus a periodic heartbeat. The
supervisor detects a dead process or a missed heartbeat and restarts the
sidecar with backoff, reloading the model.

Audio capture stays in the main process; only PCM segments cross the IPC
boundary, so the sidecar itself is stateless and freely restartable.

## Consequences

- A CUDA crash or VRAM OOM degrades to "dictation briefly unavailable while it
  restarts," not "the whole app is gone."
- Extra complexity: an IPC protocol, a supervisor with backoff logic, and two
  binaries to build instead of one.
- **The dependency direction is load-bearing:** `stt-sidecar` depends on the
  `voice-type` library (for the shared `ipc` module), never the other way
  around. If `src-tauri` ever depended on `stt-sidecar` or linked
  `whisper-rs` directly, the main crate would require the CUDA toolkit to
  build, which would break `cargo test -p voice-type` on CI (CI has no CUDA
  toolchain — see the [SDLC docs design](../superpowers/specs/2026-07-26-sdlc-docs-design.md#7-integración-continua)).

## Alternatives considered

- **Same-process, thread-isolated:** a crash in native code can still take
  down the whole process even from a worker thread; threads don't provide
  fault isolation the way a separate process does. Rejected.
- **Windows Service running the STT engine:** Session 0 (where services run)
  has no GPU/desktop access in the way an interactive session does — a known
  trap. Rejected; see [ADR 0005](0005-tauri-v2-and-svelte-stack.md) and spec §13.
