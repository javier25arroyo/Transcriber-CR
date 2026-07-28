# Voice Type

Voice dictation for Windows 11. Local, offline, GPU-accelerated speech-to-text
in Spanish, with a global toggle hotkey that types the transcription straight
into whatever application has focus.

**Status:** v1 in development. See the [implementation plan](docs/superpowers/plans/2026-07-25-voice-type-dictation.md)
and the [Changelog](CHANGELOG.md) for progress. Development happens on
`develop`; see the [release process](docs/release-process.md) for how a
version gets from there to a tagged release on `main`.

[Español ↓](#voice-type-1)

## What it is

Press a global shortcut, speak Spanish, press it again to stop. The
transcription is inserted into the focused window as you speak, segment by
segment — not all at once at the end. A small floating, translucent pill shows
listening/transcribing state and lets you correct misheard words on the spot.

## Custom vocabulary

Recognition is locked to Spanish, but real speech mixes in English jargon and
names. Tap the "+" button on the pill, pick the word Whisper got wrong, type
the correct form. That correction is used two ways: it biases future
recognition toward that term, and it deterministically fixes it if heard again.
No config files to hand-edit.

## Requirements

- Windows 11
- Rust (stable, via [rustup](https://rustup.rs/))
- Node.js LTS
- NVIDIA GPU with 8 GB+ VRAM and [CUDA Toolkit 12.x](https://developer.nvidia.com/cuda-downloads)
  (falls back to CPU automatically if CUDA isn't available — slower)

Verify your toolchain:

```bash
rustc --version
node --version
nvcc --version
```

## Quick start

```bash
pnpm install
pnpm tauri dev
```

The Whisper model downloads automatically on first run.

## How it works

Two processes, supervisor + worker, so the app stays up even if the speech
engine crashes:

```
┌─────────────────────────────────────────────┐
│  Main process (Tauri / Rust)                 │
│  - UI (WebView2 + Svelte): pill, popup, tray │
│  - Global hotkeys, autostart, single instance│
│  - Vocabulary store, post-processor, injector│
│  - Supervisor (spawns + watches the sidecar) │
└───────────────┬─────────────────────────────┘
                │ IPC: stdin/stdout JSON + heartbeat
┌───────────────┴─────────────────────────────┐
│  STT sidecar (separate process, Rust)        │
│  - whisper-rs (whisper.cpp + CUDA)           │
│  - Loads the model once, stays warm          │
└──────────────────────────────────────────────┘
```

If the sidecar crashes or runs out of VRAM, the supervisor detects it and
restarts it — the UI and your workflow keep running.

## Naming note

The GitHub repository is named `Transcriber-CR` for historical reasons; the
product is called **Voice Type**, and the Rust crate is named `voice-type`.
These are three independent identifiers — see
[docs/adr](docs/adr/) if that ever needs revisiting.

## Third-party licenses

- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) — MIT
- OpenAI's `ggml` Whisper model weights — MIT

The model itself is **not** bundled in this repository; it's downloaded at
runtime on first launch.

## More

- [Product spec](docs/superpowers/specs/2026-07-25-voice-type-dictation-design.md)
- [Implementation plan](docs/superpowers/plans/2026-07-25-voice-type-dictation.md)
- [SDLC docs design](docs/superpowers/specs/2026-07-26-sdlc-docs-design.md)
- [Architecture Decision Records](docs/adr/)
- [Contributing](CONTRIBUTING.md)

---

# Voice Type

Dictado por voz para Windows 11. Local, offline, con aceleración por GPU,
reconocimiento en español, y un atajo global tipo toggle que escribe la
transcripción directo en la aplicación que tenga el foco.

**Estado:** v1 en desarrollo. Ver el [plan de implementación](docs/superpowers/plans/2026-07-25-voice-type-dictation.md)
y el [Changelog](CHANGELOG.md) para ver el progreso. El desarrollo pasa por
`develop`; ver el [proceso de release](docs/release-process.md) para cómo una
versión llega de ahí a un tag publicado en `main`.

## Qué es

Apretás un atajo global, hablás en español, lo apretás de nuevo para parar. La
transcripción se inserta en la ventana enfocada mientras hablás, de a
segmentos, no toda junta al final. Una píldora flotante translúcida muestra el
estado (escuchando/transcribiendo) y permite corregir palabras mal
reconocidas ahí mismo.

## Vocabulario personalizado

El reconocimiento está fijado a español, pero el habla real mezcla jerga en
inglés y nombres propios. Tocás el botón "+" en la píldora, elegís la palabra
que Whisper transcribió mal, escribís la forma correcta. Esa corrección se usa
de dos formas: sesga el reconocimiento futuro hacia ese término, y lo corrige
de forma determinista si se vuelve a escuchar mal. Sin archivos de
configuración para editar a mano.

## Requisitos

- Windows 11
- Rust (estable, vía [rustup](https://rustup.rs/))
- Node.js LTS
- GPU NVIDIA con 8 GB+ de VRAM y [CUDA Toolkit 12.x](https://developer.nvidia.com/cuda-downloads)
  (si CUDA no está disponible, cae a CPU automáticamente — más lento)

Verificar el toolchain:

```bash
rustc --version
node --version
nvcc --version
```

## Arranque rápido

```bash
pnpm install
pnpm tauri dev
```

El modelo de Whisper se descarga solo en el primer arranque.

## Cómo funciona

Dos procesos, supervisor + worker, para que la app siga viva aunque el motor
de voz se caiga:

```
┌─────────────────────────────────────────────┐
│  Proceso principal (Tauri / Rust)            │
│  - UI (WebView2 + Svelte): pill, popup, tray │
│  - Atajos globales, autostart, instancia única│
│  - Vocabulario, post-procesador, inyector    │
│  - Supervisor (lanza y vigila el sidecar)    │
└───────────────┬─────────────────────────────┘
                │ IPC: stdin/stdout JSON + heartbeat
┌───────────────┴─────────────────────────────┐
│  Sidecar STT (proceso aparte, Rust)          │
│  - whisper-rs (whisper.cpp + CUDA)           │
│  - Carga el modelo una vez, se mantiene warm │
└──────────────────────────────────────────────┘
```

Si el sidecar se cae o se queda sin VRAM, el supervisor lo detecta y lo
reinicia — la UI y tu flujo de trabajo siguen andando.

## Nota sobre el nombre

El repositorio de GitHub se llama `Transcriber-CR` por motivos históricos; el
producto se llama **Voice Type**, y el crate de Rust se llama `voice-type`.
Son tres identificadores independientes — ver [docs/adr](docs/adr/) si algún
día hace falta revisar esto.

## Licencias de terceros

- [whisper.cpp](https://github.com/ggerganov/whisper.cpp) — MIT
- Pesos del modelo Whisper `ggml` de OpenAI — MIT

El modelo **no** viene incluido en este repositorio; se descarga en tiempo de
ejecución en el primer arranque.

## Más

- [Spec de producto](docs/superpowers/specs/2026-07-25-voice-type-dictation-design.md)
- [Plan de implementación](docs/superpowers/plans/2026-07-25-voice-type-dictation.md)
- [Diseño de documentos SDLC](docs/superpowers/specs/2026-07-26-sdlc-docs-design.md)
- [Registro de decisiones de arquitectura](docs/adr/)
- [Cómo contribuir](CONTRIBUTING.md)
