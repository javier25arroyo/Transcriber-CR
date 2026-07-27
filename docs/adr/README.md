# Architecture Decision Records

This directory records the architecture decisions for Voice Type that are
expensive to reverse. It does **not** duplicate the product spec — each ADR
links back to the spec section it formalizes and stays short.

## Format

Every ADR follows the same four sections:

- **Context** — what problem or constraint forced a decision.
- **Decision** — what was decided, stated plainly.
- **Consequences** — what this makes easier, what it makes harder, and what
  future work must respect.
- **Alternatives considered** — what else was on the table and why it lost.

Status is either `Accepted`, `Superseded by ADR-000X`, or `Deprecated`.

## When to write a new one

Only for decisions that are costly to reverse — a different STT engine, a
different process architecture, dropping the fixed-Spanish constraint, that
kind of thing. Ordinary implementation choices belong in code comments or PR
descriptions, not here.

## Index

| ADR | Title | Status |
|---|---|---|
| [0001](0001-isolated-stt-sidecar.md) | Isolated STT sidecar process | Accepted |
| [0002](0002-toggle-over-push-to-talk.md) | Toggle activation over push-to-talk | Accepted |
| [0003](0003-clipboard-paste-injection.md) | Clipboard + Ctrl+V text injection | Accepted |
| [0004](0004-fixed-spanish-with-custom-vocabulary.md) | Fixed Spanish recognition with custom vocabulary | Accepted |
| [0005](0005-tauri-v2-and-svelte-stack.md) | Tauri v2 + Svelte + whisper-rs stack | Accepted |
