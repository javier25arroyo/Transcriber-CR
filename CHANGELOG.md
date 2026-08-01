# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Flujo de ramas `develop` → `release/*` → `main` con tag, documentado en
  [ADR 0006](docs/adr/0006-branching-and-release-flow.md) y en
  [docs/release-process.md](docs/release-process.md). `develop` pasa a ser la
  rama por defecto y el destino de los PR de trabajo; `main` solo se mueve
  fusionando una rama `release/*` o `hotfix/*`, y cada fusión a `main` queda
  taggeada.
- `scripts/version.mjs`: sincroniza y verifica la versión entre `Cargo.toml`,
  `package.json` y `src-tauri/tauri.conf.json` (`pnpm version:check` /
  `pnpm version:set <versión>`).
- `scripts/changelog-section.mjs`: extrae de `CHANGELOG.md` la sección de una
  versión (o `[Unreleased]` si todavía no está cerrada), usado para el cuerpo
  del GitHub Release.
- `.github/workflows/release.yml`: al empujar un tag `vX.Y.Z` (o
  `vX.Y.Z-rc.N`), publica el GitHub Release correspondiente con las notas
  tomadas del CHANGELOG.
- CI: job `versioning` que falla si las tres fuentes de versión no coinciden
  entre sí, o si una rama `release/*`/`hotfix/*` no coincide con la versión de
  sus manifiestos o no tiene ya una sección cerrada en el CHANGELOG — el "paso
  antes del tag" hecho verificable.
- `.mcp.json`: registro del servidor MCP `codebase-memory-mcp` para el
  proyecto (herramienta opcional para agentes de código, no forma parte del
  build de la app). Ver CONTRIBUTING.md § Codebase memory MCP.
- Scaffold del proyecto: workspace de Cargo con el paquete `voice-type`
  (librería + binario Tauri v2) y frontend Svelte 5 + Vite + TypeScript.
- Features de cargo `app` e `injector`, que aíslan las dependencias de Tauri y
  del portapapeles para poder compilar y testear la lógica fuera de Windows.
- Crate `stt-sidecar` con el loop de IPC, el heartbeat y el manejo del
  protocolo contra el trait `Engine`. El motor real entra con la feature
  `whisper`, apagada por defecto.
- Motor STT real: `WhisperEngine` implementa el trait `Engine` sobre
  `whisper-rs` 0.16 (whisper.cpp), detrás de la feature `whisper`. Carga el
  modelo, lo mantiene caliente y transcribe cada segmento; `build_engine()` lo
  usa cuando el sidecar se compila con la feature y cae a `UnavailableEngine`
  si no. Se compila con el backend CPU de whisper.cpp (sin la feature `cuda`);
  prender CUDA es cambiar una línea de la dependencia.
- Ejemplo `dictation_probe` (features `whisper` + `capture`): dicta de punta a
  punta en la terminal — micrófono → segmentador → `WhisperEngine` → texto —
  para probar el motor sin levantar la app Tauri.
- Test de integración `transcribes_reference_wav` (`#[ignore]`): transcribe un
  wav de voz sintética y verifica el texto contra el modelo local.
- `scripts/build-whisper.ps1`: descubre LLVM (libclang), CMake y los headers de
  MSVC/Windows SDK, y corre cargo con el entorno que la feature `whisper`
  necesita en Windows.
- `scripts/generate-test-fixtures.ps1`: regenera el fixture de audio del test de
  integración con la síntesis de voz de Windows (SAPI), a 16 kHz mono.
- Módulo `pipeline`: qué se escribe en la ventana por cada segmento
  transcripto.
- Módulo `hotkeys`: `DictationState` y la semántica del toggle.
- Módulo `model_download`: rutas, URL y validación del nombre del modelo.
- Píldora flotante y popup de vocabulario en el frontend, con los iconos de la
  app generados desde `app-icon.svg`.
- Módulo `vocab_store`: términos y correcciones del vocabulario, con
  persistencia en JSON.
- Módulo `post_processor`: reemplazos por palabra completa y unión de segmentos.
- Módulo `ipc`: protocolo entre el proceso principal y el sidecar, una línea
  JSON por mensaje.
- Módulo `config`: ajustes persistentes (atajo, modelo, micrófono, posición de
  la píldora).
- Módulo `vad`: segmentación por energía y silencio, con corte máximo de
  segmento.
- Módulo `supervisor`: lanzamiento del sidecar, canales de IPC, backoff y
  watchdog que también detecta al proceso vivo pero mudo.
- Módulo `text_injector`: guardado y restauración del portapapeles alrededor
  del Ctrl+V.
- Módulo `audio`: downmix a mono y resample a 16 kHz, con un resampler continuo
  que no pierde audio entre bloques.
- Captura real de micrófono (`AudioCapture`, feature `capture`) con `cpal`,
  más el ejemplo `mic_probe` para verificarla a mano.
- Bitácora de progreso en `docs/progress.md`.

### Changed

- CONTRIBUTING.md, la plantilla de PR y README.md: el flujo de trabajo pasa a
  abrir PRs contra `develop` en vez de `main` (ver ADR 0006).
- CI: los triggers de `push`/`pull_request` ahora cubren `develop`,
  `release/**` y `hotfix/**` además de `main`.
- CI: se retiraron las guardas temporales de repo vacío, se construye el
  frontend antes de compilar el crate en Windows y se agregó un job en Linux
  que corre la lógica pura con `--no-default-features` y revisa el módulo del
  portapapeles con `--features injector`.
- Gestor de paquetes del frontend: de npm a pnpm (`pnpm@9.15.4`, fijado en
  `package.json` → `packageManager`). `README.md` y `CONTRIBUTING.md` usan los
  comandos de pnpm; `package-lock.json` salió del control de versiones a favor
  de `pnpm-lock.yaml`.

### Fixed

- CI: los tres jobs que instalan Node.js seguían configurados para npm
  (`cache: npm`, `npm ci`) después de la migración a pnpm, lo que los hacía
  fallar apenas `package-lock.json` dejó de existir. Se agregó
  `pnpm/action-setup` antes de `actions/setup-node` en los jobs `rust` y
  `frontend`, con `cache: pnpm`, y los comandos pasaron a
  `pnpm install --frozen-lockfile` / `pnpm build` / `pnpm test` / `pnpm check`.
