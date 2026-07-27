# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Scaffold del proyecto: workspace de Cargo con el paquete `voice-type`
  (librería + binario Tauri v2) y frontend Svelte 5 + Vite + TypeScript.
- Features de cargo `app` e `injector`, que aíslan las dependencias de Tauri y
  del portapapeles para poder compilar y testear la lógica fuera de Windows.
- Crate `stt-sidecar` con el loop de IPC, el heartbeat y el manejo del
  protocolo contra el trait `Engine`. El motor real entra con la feature
  `whisper`, apagada por defecto.
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
- Bitácora de progreso en `docs/progress.md`.

### Changed

- CI: se retiraron las guardas temporales de repo vacío, se construye el
  frontend antes de compilar el crate en Windows y se agregó un job en Linux
  que corre la lógica pura con `--no-default-features` y revisa el módulo del
  portapapeles con `--features injector`.

### Fixed
