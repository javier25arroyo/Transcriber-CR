# Bitácora de progreso

Estado de las tareas del [plan de implementación](superpowers/plans/2026-07-25-voice-type-dictation.md),
qué se probó en cada una y qué queda pendiente de verificar a mano.

El plan es la fuente de verdad de *qué* hay que hacer (con sus checkboxes); este
documento es el registro de *qué se ejecutó y con qué resultado*.

## Entorno de desarrollo de esta tanda

El trabajo se hizo en un contenedor **Linux sin GPU NVIDIA, sin WebKitGTK y sin
ALSA**. El objetivo del producto sigue siendo Windows 11. Consecuencia práctica:

- Todo lo que es lógica pura se implementó con TDD y se ejecutó de verdad, vía
  `cargo test -p voice-type --no-default-features` (la feature `app` apaga la
  dependencia de Tauri; ver CONTRIBUTING.md).
- El frontend se ejecutó completo: `vitest`, `svelte-check` y `vite build`.
- Lo que necesita Windows, GPU o hardware (abrir la ventana de Tauri, micrófono,
  portapapeles real, CUDA) queda marcado abajo como **pendiente de verificación
  manual**, sin darlo por hecho.

## Estado por tarea

| Task | Módulo | Estado | Verificación |
|---|---|---|---|
| 1 | Scaffold Tauri v2 + Svelte | Hecho (falta Step 3) | `vite build`, `vitest`, `svelte-check`, `cargo test --no-default-features` |
| 2 | `vocab_store` | Pendiente | — |
| 3 | `post_processor` | Pendiente | — |
| 4 | `ipc` | Pendiente | — |
| 5 | `config` | Pendiente | — |
| 6 | Sidecar STT (whisper-rs) | Pendiente | Requiere CUDA |
| 7 | `audio` | Pendiente | Requiere micrófono |
| 8 | `vad` | Pendiente | — |
| 9 | `supervisor` | Pendiente | — |
| 10 | `text_injector` | Pendiente | Requiere Windows |
| 11 | `pipeline` | Pendiente | — |
| 12 | Hotkeys + comandos | Pendiente | Requiere Windows |
| 13 | `PillWidget` | Pendiente | Verificación visual |
| 14 | `AddWordPopup` | Pendiente | Verificación visual |
| 15 | Tray, autostart, modelo | Pendiente | Requiere Windows |

## Detalle

### Task 1 — Scaffold del proyecto

**Qué se hizo.** Workspace de Cargo con `src-tauri` (paquete `voice-type`,
librería `voice_type` + binario), frontend Svelte 5 + Vite + TypeScript, y la
configuración de Tauri v2 (`productName: "Voice Type"`,
`identifier: "com.javier.voicetype"`).

El scaffold se escribió a mano en vez de usar `npm create tauri-app`: el
generador es interactivo y la carpeta ya tenía `docs/`, `.github/` y los
documentos de SDLC que no había que pisar.

**Decisión de implementación: la feature `app`.** La dependencia de `tauri` y
`tauri-build` está detrás de una feature de cargo activada por defecto. Con
`--no-default-features` se compila sólo la librería de módulos puros, sin
toolkit gráfico ni frontend construido. Esto permite correr los tests de la
lógica determinista en cualquier máquina —incluido el runner Linux del CI— y no
sólo en la máquina Windows de desarrollo. `build.rs` respeta la misma feature.

**CI.** Se retiraron las guardas `hashFiles(...)` temporales (Step 4 del plan) y
se agregó un job `core-logic` en Linux que corre clippy y los tests con
`--no-default-features`. Ese job es el que detecta si un módulo que debería ser
puro empieza a depender de Tauri. El job de Windows ahora construye el frontend
antes de compilar, porque `tauri-build` exige que exista `dist/`.

**Pruebas ejecutadas.**

```
npx vitest run        → 2 passed (store de estado)
npx svelte-check      → 0 errors, 0 warnings
npm run build         → OK
cargo fmt --all --check                                   → OK
cargo clippy -p voice-type --no-default-features --all-targets -- -D warnings → sin warnings
cargo test -p voice-type --no-default-features             → OK
```

**Pendiente de verificación manual (Windows 11).** Step 3 del plan:
`npm run tauri dev` tiene que abrir la ventana. El bundle está desactivado
(`bundle.active: false`) hasta la Task 15, que es la que agrega los iconos.
