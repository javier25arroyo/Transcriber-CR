# Security Policy

## What this app does

Voice Type is a background dictation service. To work, it needs to:

- Capture continuous audio from your microphone.
- Read and overwrite the **system clipboard** momentarily during every text
  injection.
- Simulate keyboard input (`Ctrl+V`) into whatever window has focus.

## What this app does not do

- No network calls except downloading the Whisper model from Hugging Face on
  first run.
- No telemetry, no analytics, no accounts, no cloud sync.
- Transcription happens entirely on-device.

## Known risk areas

- **Clipboard exposure window:** during text injection, your existing
  clipboard content is temporarily replaced, then restored. A third-party
  clipboard manager running at the same time could capture the transcribed
  text during that window.
- **Focus-follows-injection:** the transcription is pasted into whichever
  window has focus at the moment of injection. If focus changes between when
  you spoke and when the segment finishes transcribing, the text can land in
  the wrong window.
- **Logs must never contain transcriptions or raw audio.** This is a hard
  implementation requirement, not a suggestion — if you find a code path that
  logs either, that's a security bug, please report it.

## Supported versions

Only the latest released version is supported. This project is pre-1.0; there
is no long-term support branch.

## Reporting a vulnerability

Please use [GitHub Security Advisories](https://github.com/javier25arroyo/Transcriber-CR/security/advisories/new)
for this repository — that's a private channel, not a public issue.

If you'd rather email, use javier25arojas@gmail.com.

Please don't open a public issue for security reports; everything else
(bugs, feature requests) is fine as a regular issue.
