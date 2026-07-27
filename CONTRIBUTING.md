# Contributing to Voice Type

Thanks for taking a look. This is currently a solo project in active early
development, but it's built to be contributable.

## Setup

1. Install [Rust](https://rustup.rs/) (stable) and [Node.js LTS](https://nodejs.org/).
2. For the STT sidecar, install the [CUDA Toolkit 12.x](https://developer.nvidia.com/cuda-downloads)
   (an NVIDIA GPU with 8 GB+ VRAM is required to build and run it; CPU fallback
   exists at runtime but isn't required to build).
3. Verify your toolchain:

   ```bash
   rustc --version
   node --version
   nvcc --version
   ```

4. Install frontend dependencies and start the dev app:

   ```bash
   npm install
   npm run tauri dev
   ```

## Project layout

```
Voice Type/
├─ src-tauri/     # main process: Tauri app, lib of pure modules
├─ stt-sidecar/   # isolated STT worker process (whisper-rs + CUDA)
├─ src/           # Svelte frontend
└─ docs/          # spec, implementation plan, ADRs
```

`src-tauri` is both a binary and a library (`voice_type`). `stt-sidecar`
depends on `voice-type` as a library to reuse the shared `ipc` module.

**This dependency direction matters and must not be reversed.** `stt-sidecar`
is the only crate that needs to compile `whisper-rs` with CUDA. If
`src-tauri` ever depended on `stt-sidecar` (or pulled in `whisper-rs`
directly), the whole main crate would require the CUDA toolkit to build —
which would break `cargo test -p voice-type` on CI, since CI has no CUDA
toolchain. See [ADR 0001](docs/adr/0001-isolated-stt-sidecar.md).

## Conventions

- Rust edition 2021, formatted with `rustfmt` defaults, `clippy` clean with no
  warnings.
- Commit messages: `feat:`, `fix:`, `test:`, `chore:`, `docs:` prefixes.
- Test-driven development for pure, deterministic logic: `vocab_store`,
  `post_processor`, `ipc`, `config`, `vad`. Write the failing test first.
- One branch per unit of work, PR into `main` (see [Workflow](#workflow)
  below).
- If you touch `README.md`, update **both** the English and Spanish sections
  in the same commit — the PR template has a checkbox for this.
- Update `CHANGELOG.md` under `[Unreleased]` in the same PR that introduces
  the change.

## Running tests

Three tiers, from cheapest to most expensive:

| Command | What it covers | Runs on CI? |
|---|---|---|
| `cargo test -p voice-type` | Pure logic in the main crate — no hardware, no GPU | Yes |
| `npx vitest run` | Frontend unit tests | Yes |
| `cargo test -p stt-sidecar -- --ignored` | Real transcription against a local model, requires CUDA and a downloaded model | No — local only |

## Manual verification

Some things can't be meaningfully automated and are checked by hand before
merging a PR that touches them:

- Microphone capture (`audio` module) — run the `mic_probe` example, confirm
  RMS rises when you speak.
- Text injection into a focused window (`text_injector` module) — run the
  `inject_probe` example against Notepad.
- Actual GPU transcription — confirm `whisper-rs` logs show the CUDA backend,
  not CPU fallback.
- Sidecar crash recovery — kill the sidecar process from Task Manager while
  dictating, confirm the supervisor restarts it and dictation keeps working.
- Autostart at Windows login.

If your change touches one of these areas, say in the PR description how you
verified it manually — the CI template has a field for this.

## Workflow

- One branch per task: `task/NN-short-description` for implementation-plan
  tasks, `fix/...` or `feat/...` for everything else.
- Open a PR into `main`. CI must pass (see below) before merging.
- Squash merge, so each unit of work is one commit on `main`.
- No mandatory reviewer for now — this is a one-person project. That
  requirement gets added the day a second contributor shows up.

## Regenerating test fixtures

The STT integration test fixture (`stt-sidecar/tests/fixtures/*.wav`) is
synthetic speech generated with Windows' built-in TTS, not a real recording —
this keeps the repo free of anyone's actual voice. Regenerate it with:

```powershell
./scripts/generate-test-fixtures.ps1
```

## Code of Conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).

## Security

Found a vulnerability? See [SECURITY.md](SECURITY.md) — please don't open a
public issue for it.
