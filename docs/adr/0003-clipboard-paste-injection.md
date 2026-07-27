# ADR 0003: Clipboard + Ctrl+V text injection

- **Status:** Accepted
- **Date:** 2026-07-26
- **Source:** [Product spec §5, §12](../superpowers/specs/2026-07-25-voice-type-dictation-design.md#12-resiliencia-y-manejo-de-errores)

## Context

Transcribed text needs to land in whatever application currently has focus —
Word, a browser, a chat client — without a dedicated Voice Type window to
type into. Two viable mechanisms exist on Windows: simulating individual
keystrokes for every character, or writing the text to the clipboard and
simulating a single `Ctrl+V`.

## Decision

Text injection saves the current clipboard contents, writes the transcribed
text to the clipboard, simulates `Ctrl+V`, then restores the original
clipboard contents.

## Consequences

- Correctly handles accented characters, `ñ`, and arbitrary Unicode without
  per-character keymap issues — simulating raw keystrokes for Spanish
  diacritics is unreliable across keyboard layouts.
- A brief window exists where the system clipboard holds the transcribed text
  instead of the user's previous content; a third-party clipboard manager
  running concurrently could capture it. Documented in
  [SECURITY.md](../../SECURITY.md).
- If the target application fails to process the paste (e.g. it doesn't have
  focus after all, or blocks paste), the clipboard is still restored — the
  user doesn't lose their previous clipboard contents even on failure.
- Requires a short real delay between setting the clipboard and restoring it,
  so the target app has time to process the paste before the original
  content comes back.

## Alternatives considered

- **Simulated keystrokes per character:** avoids touching the clipboard at
  all, but Unicode input (accents, `ñ`) is unreliable this way across
  different keyboard layouts and target applications. Rejected.
- **Accessibility API (UI Automation) text insertion:** would avoid both the
  clipboard and keystroke simulation, but not all target applications
  (particularly Electron-based chat apps, some browser text areas) implement
  the necessary UIA text patterns consistently. Rejected for v1 as
  higher-risk with less universal coverage.
