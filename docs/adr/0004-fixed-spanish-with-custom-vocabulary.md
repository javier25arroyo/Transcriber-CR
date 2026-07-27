# ADR 0004: Fixed Spanish recognition with custom vocabulary

- **Status:** Accepted
- **Date:** 2026-07-26
- **Source:** [Product spec §3, §9](../superpowers/specs/2026-07-25-voice-type-dictation-design.md#9-vocabulario-personalizado)

## Context

The primary user dictates in Spanish but regularly mixes in English technical
jargon and proper nouns (tool names, library names, acronyms). Whisper
supports automatic language detection and multilingual transcription, but
auto-detection adds latency and can misfire mid-sentence when the spoken
language actually switches within a segment.

## Decision

The recognition language is fixed to `"es"`. English terms are handled
through a **custom vocabulary** mechanism instead of language auto-detection:
user-added terms are passed to whisper.cpp as an `initial_prompt` to bias
recognition toward them, and a deterministic `heard → correct` replacement map
fixes whatever the bias doesn't catch. Both are fed from the same "+" button
flow in the UI.

## Consequences

- Simpler, more predictable recognition behavior — no per-segment language
  guessing, no risk of a segment being transcribed in the wrong language
  entirely.
- English jargon accuracy depends entirely on the user having added the term
  before speaking it for the first time; first-use accuracy for new terms is
  not solved by this design.
- Whisper.cpp has no formal "hotwords" parameter — the bias mechanism is
  `initial_prompt`, which is a soft nudge, not a guarantee. This is why the
  deterministic replacement map exists as a second, hard-correcting layer.
- Adding vocabulary must reload `initial_prompt` in the sidecar without a full
  model reload, or the correction loop feels too slow to use in practice.

## Alternatives considered

- **Automatic multilingual detection:** more "magic," but per-segment
  language switching risk and added latency made it a worse fit for a
  fast, low-friction dictation loop. Rejected for v1; noted as a possible
  direction if usage patterns change.
- **Static vocabulary file edited by hand:** rejected outright — the whole
  point of the "+" button is that the user never opens a config file.
