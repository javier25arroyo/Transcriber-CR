# ADR 0002: Toggle activation over push-to-talk

- **Status:** Accepted
- **Date:** 2026-07-26
- **Source:** [Product spec §3, §8.1](../superpowers/specs/2026-07-25-voice-type-dictation-design.md#8-flujos-de-datos)

## Context

Voice Type needs a way to start and stop listening that works from any
application, without a dedicated window to hold focus. Two common patterns
exist: push-to-talk (hold a key while speaking) and toggle (press once to
start, press again to stop).

## Decision

Activation is a **toggle**, bound to a single global hotkey. One press starts
listening; a second press stops it and flushes the last pending segment.

## Consequences

- Hands stay free while dictating — no key needs to be held down through a
  whole sentence or paragraph, which matters for longer dictation.
- The app must track and clearly surface state (idle / listening /
  transcribing) since there's no physical key-held cue telling the user
  whether it's still listening — this is why the pill widget shows an
  animated state indicator.
- Forgetting to toggle off is a real failure mode: the app can keep capturing
  audio the user didn't intend to dictate. Mitigated by a visible,
  always-on-top pill.

## Alternatives considered

- **Push-to-talk:** simpler state model (listening only while the key is
  physically held), but impractical for dictating anything longer than a
  short phrase, and this tool's stated purpose is replacing sustained typing.
  Rejected for v1 — noted in spec §16 as a possible v2 alternative mode, not a
  replacement.
- **Dedicated dictation window that must stay focused:** defeats the purpose
  of inserting text into whatever app the user is actually working in.
  Rejected.
