# Contributing to Voice Type

Thanks for taking a look. This is currently a solo project in active early
development, but it's built to be contributable.

## Setup

1. Install [Rust](https://rustup.rs/) (stable), [Node.js LTS](https://nodejs.org/) and
   [pnpm](https://pnpm.io/installation) (the version pinned in `package.json`'s
   `packageManager` field — `corepack enable` picks it up automatically on
   recent Node).
2. For the STT sidecar's `whisper` feature, `whisper-rs` builds whisper.cpp
   from source, so it needs a C/C++ toolchain that cargo alone won't find on a
   fresh Windows box:
   - [LLVM](https://github.com/llvm/llvm-project/releases) (`winget install LLVM.LLVM`) —
     `libclang`, used by bindgen to generate the whisper.cpp bindings.
   - [CMake](https://cmake.org/download/) (`winget install Kitware.CMake`) —
     builds the whisper.cpp static library.
   - Visual Studio 2022 with the C++ toolset (MSVC + the Windows SDK).

   You don't have to remember the include paths: `scripts/build-whisper.ps1`
   discovers them and runs cargo with the right environment (see
   [Running tests](#running-tests)). The `whisper` feature builds with the CPU
   backend by default; the [CUDA Toolkit 12.x](https://developer.nvidia.com/cuda-downloads)
   (NVIDIA GPU, 8 GB+ VRAM) is only needed to build it with GPU acceleration.
3. Verify your toolchain:

   ```bash
   rustc --version
   node --version
   ```

4. Install frontend dependencies and start the dev app:

   ```bash
   pnpm install
   pnpm tauri dev
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

`whisper-rs` sits behind `stt-sidecar`'s `whisper` cargo feature, off by
default. The sidecar's protocol handling talks to an `Engine` trait, so it
compiles and is tested without the CUDA toolkit; production builds use
`cargo build -p stt-sidecar --features whisper --release`.

The Tauri dependency itself sits behind the `app` cargo feature (on by
default). Turning it off — `--no-default-features` — builds only the library:
the pure, deterministic modules, with no GUI toolkit and no built frontend
required. That is what makes the modules testable on a machine that isn't the
Windows target box.

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
- One branch per unit of work, PR into `develop` (see [Workflow](#workflow)
  below).
- If you touch `README.md`, update **both** the English and Spanish sections
  in the same commit — the PR template has a checkbox for this.
- Update `CHANGELOG.md` under `[Unreleased]` in the same PR that introduces
  the change.

## Running tests

Three tiers, from cheapest to most expensive:

| Command | What it covers | Runs on CI? |
|---|---|---|
| `cargo test -p voice-type --no-default-features` | Pure logic only — no Tauri, no built frontend, runs on any OS | Yes (Linux job) |
| `cargo test -p voice-type` | The same tests plus everything the Tauri app pulls in; needs `pnpm build` first | Yes (Windows job) |
| `pnpm test` | Frontend unit tests | Yes |
| `cargo test -p stt-sidecar` | The sidecar's protocol handling, against a fake engine — no whisper build needed | Yes (Linux job) |
| `./scripts/build-whisper.ps1 test -p stt-sidecar --features whisper -- --ignored` | Real transcription against a local model — needs the C/C++ toolchain and a downloaded model | No — local only |

The first row is the one to reach for while working on a module: it skips the
`app` feature, so `cargo` never has to build Tauri or find a WebView. The
Windows job is what proves the app itself still compiles.

Anything that touches the `whisper` feature has to go through
`scripts/build-whisper.ps1` on Windows (it sets `LIBCLANG_PATH`,
`BINDGEN_EXTRA_CLANG_ARGS` and puts CMake on `PATH`); a plain
`cargo build --features whisper` fails because bindgen can't find the CRT
headers on its own. The integration test needs `models/ggml-base.bin` — download
it once from
[Hugging Face](https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin)
into `models/` (git-ignored).

## Manual verification

Some things can't be meaningfully automated and are checked by hand before
merging a PR that touches them:

- Microphone capture (`audio` module) — run the `mic_probe` example, confirm
  RMS rises when you speak.
- Text injection into a focused window (`text_injector` module) — run the
  `inject_probe` example against Notepad.
- End-to-end dictation — run the `dictation_probe` example and speak; confirm
  the recognised text prints:

  ```powershell
  ./scripts/build-whisper.ps1 run -p stt-sidecar --example dictation_probe --features "whisper capture" -- models/ggml-base.bin es
  ```

- Actual GPU transcription — build the `whisper` feature with CUDA and confirm
  `whisper-rs` logs show the CUDA backend, not CPU fallback.
- Sidecar crash recovery — kill the sidecar process from Task Manager while
  dictating, confirm the supervisor restarts it and dictation keeps working.
- Autostart at Windows login.

For the pill and the popup there's a cheaper first pass that doesn't need
Windows: `pnpm build && pnpm preview` renders them in any browser. The
Tauri calls fail there (there's no backend), which the components handle, so
you get the layout, the colors and the light/dark variants without leaving the
dev machine. What that pass can't tell you is whether the translucency and the
always-on-top window behave on real Windows — that still needs the target box.

If your change touches one of these areas, say in the PR description how you
verified it manually — the CI template has a field for this.

## Workflow

- One branch per task: `task/NN-short-description` for implementation-plan
  tasks, `fix/...` or `feat/...` for everything else — cut from `develop`.
- Open a PR into `develop`. CI must pass (see below) before merging.
- Squash merge, so each unit of work is one commit on `develop`.
- No mandatory reviewer for now — this is a one-person project. That
  requirement gets added the day a second contributor shows up.
- `main` only moves via a `release/*` or `hotfix/*` branch, merged with a
  merge commit and tagged. See [ADR 0006](docs/adr/0006-branching-and-release-flow.md)
  and [docs/release-process.md](docs/release-process.md) for the full flow,
  including the manual verification checklist that has to pass before a
  release reaches `main`.

## Codebase memory MCP (optional, for coding agents)

`.mcp.json` at the repo root registers [`codebase-memory-mcp`](https://github.com/DeusData/codebase-memory-mcp)
as a project-scoped MCP server: a local code-intelligence index (call graphs,
symbol search) that Claude Code and similar agents can query instead of
re-grepping the tree on every task. It's optional tooling for whoever is
driving an agent against this repo — nothing in the Voice Type app depends on
it, and it's not part of the build.

Install the binary yourself; the repo only carries the pointer to it:

```bash
curl -fsSL https://raw.githubusercontent.com/DeusData/codebase-memory-mcp/main/install.sh | bash -s -- --skip-config
```

`--skip-config` matters. Without it, the installer's own `install` step edits
the configuration of every supported coding-agent client it finds on your
account (43 of them, per its own docs) — broader than "just this project" and
not something to run unreviewed. `.mcp.json` here is the project-scoped,
diffable alternative: it only takes effect for whoever opens this repo, and
you can see exactly what it does before trusting it.

The install script downloads a prebuilt archive and verifies its SHA-256
against a `checksums.txt` published alongside it before running anything —
worth knowing, but that only proves the download wasn't corrupted or swapped
in transit, not that the tool itself is trustworthy. Review it with the same
scrutiny you'd give any third-party binary before wiring it into your agent's
toolchain.

Make sure the install directory is on your `PATH` (the installer warns if it
isn't; default is `~/.local/bin`), then restart your coding agent so it picks
up `.mcp.json`.

## Regenerating the app icons

`src-tauri/icons/` is generated from `app-icon.svg` at the repo root:

```bash
pnpm tauri icon app-icon.svg -o src-tauri/icons
```

The generator also emits macOS, iOS and Android variants; delete them. v1 targets
Windows only, and `icons/icon.ico` is the one `tauri-build` actually requires —
without it the Windows build fails even with bundling turned off.

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
