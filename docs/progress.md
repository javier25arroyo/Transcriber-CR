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
| 2 | `vocab_store` | Hecho | 6 tests unitarios |
| 3 | `post_processor` | Hecho | 9 tests unitarios |
| 4 | `ipc` | Hecho | 5 tests unitarios |
| 5 | `config` | Hecho | 6 tests unitarios + smoke test del sidecar |
| 6 | Sidecar STT (whisper-rs) | Pendiente | Requiere CUDA |
| 7 | `audio` | Pendiente | Requiere micrófono |
| 8 | `vad` | Hecho | 8 tests unitarios |
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

### Task 2 — `vocab_store`

**Qué se hizo.** `Vocab` con `terms` (sesgo para Whisper) y `replacements`
(correcciones deterministas), persistencia en JSON y `initial_prompt()`.

**Desvío respecto del plan.** El test del plan pide que agregar dos veces la
misma corrección deje un solo reemplazo, pero el snippet de implementación del
propio plan hacía `push` incondicional. Ganó el test: `add` ahora indexa por
`heard` (case-insensitive, igual que el `post_processor` que lo consume) y pisa
el destino anterior en vez de acumular una entrada muerta. Se agregaron dos
tests que fijan ese comportamiento: pisar el destino de un `heard` conocido, y
conservar varias formas mal escuchadas del mismo término.

**Pruebas ejecutadas.**

```
cargo test -p voice-type --no-default-features vocab_store  → 6 passed
cargo clippy ... -D warnings                                → sin warnings
```

### Task 3 — `post_processor`

**Qué se hizo.** `apply()` (reemplazos del vocabulario por palabra completa,
sin distinguir mayúsculas) y `join_segments()` (une segmentos y capitaliza
después de `.`, `!` o `?`).

**Desvío respecto del plan.** El snippet del plan indexaba el texto original con
posiciones calculadas sobre su versión en minúsculas. En español eso rompe: `ó`
ocupa dos bytes y `to_lowercase()` puede cambiar el largo, así que el corte cae
en medio de un carácter y revienta. La implementación recorre caracteres, no
bytes, y hay un test con `CANCIÓN` que fija el caso. También se cubrió el
`heard` vacío, que con el algoritmo del plan era un bucle infinito.

**Pruebas ejecutadas.**

```
cargo test -p voice-type --no-default-features post_processor  → 9 passed
cargo clippy ... -D warnings                                   → sin warnings
```

### Task 4 — `ipc`

**Qué se hizo.** `ToSidecar` (LoadModel/Transcribe/Shutdown), `FromSidecar`
(Ready/Result/Heartbeat/Error) y el par `encode`/`decode` con framing de una
línea JSON por mensaje.

`decode` recorta el salto final, así que `decode(&encode(&msg))` funciona sin
que quien llama tenga que acordarse de hacer `.trim()`. Hay un test que fija que
un texto con salto de línea adentro no parte el framing.

**Anotado para el futuro.** Mandar el PCM como números en JSON es caro (unos
80 000 valores por segmento de 5 s). Se deja así porque el canal es una tubería
local y serializar cuesta mucho menos que transcribir; si alguna vez aparece
como cuello de botella, pasar a framing binario merece su propio ADR.

**Pruebas ejecutadas.**

```
cargo test -p voice-type --no-default-features ipc  → 5 passed
cargo clippy ... -D warnings                        → sin warnings
```

### Task 5 — `config` y crate `stt-sidecar`

**Qué se hizo.** `AppConfig` (atajo, modelo, micrófono, posición de la píldora)
con load/save en JSON, y el crate `stt-sidecar` cableado como miembro del
workspace.

**Endurecimiento sobre el plan.** El struct lleva `#[serde(default)]`: un
archivo de config escrito por una versión anterior no tiene los campos que se
agreguen después, y sin eso serde falla al deserializar y `load` termina
descartando *todos* los ajustes del usuario por un campo que falta. Hay un test
que carga `{"mic_device":"Mic viejo"}` y verifica que conserva el micrófono y
completa el resto con los valores por defecto.

**Dirección de la dependencia.** `stt-sidecar` depende de `voice-type` con
`default-features = false`, que apaga la feature `app`. Verificado con
`cargo tree`: en el árbol del sidecar no aparece Tauri. Es la mitad concreta de
lo que exige el ADR 0001.

El `main.rs` del sidecar trae el loop de IPC y el heartbeat completos; el motor
de Whisper se enchufa en la Task 6, y hasta entonces `LoadModel` y `Transcribe`
responden `Error`, que es justo lo que el supervisor sabe manejar.

**Pruebas ejecutadas.**

```
cargo test -p voice-type --no-default-features config  → 6 passed
cargo test -p voice-type --no-default-features         → 26 passed (total acumulado)
cargo clippy -p stt-sidecar --all-targets -- -D warnings → sin warnings
cargo tree -p stt-sidecar                              → sin Tauri en el árbol
```

Smoke test del loop de IPC, mandándole cuatro líneas por stdin
(LoadModel, una línea basura, Transcribe, Shutdown):

```
{"type":"Error","message":"motor STT todavía no implementado (Task 6 del plan)"}
{"type":"Error","message":"motor STT todavía no implementado (Task 6 del plan)"}
```

La línea basura se ignora sin tirar el proceso y `Shutdown` corta limpio.

### Task 8 — `vad`

**Qué se hizo.** `Segmenter`: acumula bloques de audio y cierra un segmento
cuando detecta silencio suficiente después de que hubo voz. Umbral por energía
(RMS 0.01) y `flush()` para el corte final cuando se apaga el toggle.

**Agregados sobre el plan.**

- *El silencio previo a la voz se descarta.* No aporta a la transcripción y sólo
  hace más lento el paso por el modelo. El silencio del final sí se conserva:
  le da a Whisper el contexto de que la frase terminó.
- *Corte máximo de 15 s.* Hablar sin pausas hacía crecer el buffer sin límite en
  la versión del plan. Whisper trabaja con una ventana de 30 s, así que un
  segmento más largo se transcribiría truncado.

**Pruebas ejecutadas.**

```
cargo test -p voice-type --no-default-features vad  → 8 passed
cargo clippy ... -D warnings                        → sin warnings
```
