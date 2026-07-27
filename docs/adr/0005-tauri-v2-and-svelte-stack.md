# ADR 0005: Tauri v2 + Svelte + whisper-rs stack

- **Status:** Accepted
- **Date:** 2026-07-26
- **Source:** [Product spec §5](../superpowers/specs/2026-07-25-voice-type-dictation-design.md#5-stack-tecnológico)

## Context

Voice Type needs to run as a stable, low-overhead background service on
Windows 11 with a translucent, animated floating UI, tight integration with
OS-level features (global hotkeys, autostart, clipboard, simulated input, tray
icon), and a GPU-accelerated local speech engine — all for personal,
single-machine use.

## Decision

- **Core / service:** Rust + Tauri v2 — single binary, memory-safe, native
  Windows integration, mature plugin ecosystem for autostart, global
  shortcuts, single-instance enforcement, and clipboard access.
- **Frontend:** Svelte — small bundle, clean CSS-driven animation, no
  framework overhead for what is essentially two small components (a pill and
  a popup).
- **STT:** `whisper-rs` (bindings over `whisper.cpp`) with the `cuda` feature
  — self-contained, no Python runtime dependency, proven stable on Windows,
  GPU-capable.
- Autostart runs at **user login** via `tauri-plugin-autostart`, explicitly
  **not** as a Windows Service — see Alternatives below.

## Consequences

- Single native binary for the main process; no bundled runtime like
  Electron's Chromium/Node duplication.
- Ties the project to Rust's ecosystem maturity for each OS integration point
  (global shortcuts, autostart, clipboard) — acceptable since mature plugins
  already exist for all of them.
- Building `whisper-rs` with CUDA on Windows requires the correct CUDA
  toolchain installed locally; acceptable because this is a single-machine
  personal build, not something distributed as a signed installer in v1.
- Session-based autostart means the app only runs while the user is logged
  in interactively — which is required anyway, since it needs GPU and
  desktop access that Session 0 (where Windows Services run) does not have.

## Alternatives considered

- **Electron:** far larger bundle and memory footprint for a background
  service meant to stay resident all day; rejected on overhead grounds.
- **Python + a GUI toolkit, calling whisper.cpp via bindings:** would add a
  Python runtime dependency and packaging complexity for what needs to be a
  lightweight always-on background process. Rejected.
- **Windows Service hosting the whole app:** Session 0 isolation means no GPU
  and no desktop/UI access from a service process — this is a well-known trap,
  not a viable path. Rejected; see spec §13 and
  [ADR 0001](0001-isolated-stt-sidecar.md).
