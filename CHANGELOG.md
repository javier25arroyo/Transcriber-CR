# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Scaffold del proyecto: workspace de Cargo con el paquete `voice-type`
  (librería + binario Tauri v2) y frontend Svelte 5 + Vite + TypeScript.
- Feature de cargo `app` (activada por defecto) que aísla la dependencia de
  Tauri, para poder testear la lógica pura sin toolkit gráfico.
- Bitácora de progreso en `docs/progress.md`.

### Changed

- CI: se retiraron las guardas temporales de repo vacío, se construye el
  frontend antes de compilar el crate en Windows y se agregó un job en Linux
  que corre la lógica pura con `--no-default-features`.

### Fixed
