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
| 6 | Sidecar STT (whisper-rs) | Preparada: protocolo hecho | 6 tests contra un motor de mentira; falta el motor real |
| 7 | `audio` | Parcial: conversión hecha | 10 tests unitarios; captura pendiente |
| 8 | `vad` | Hecho | 8 tests unitarios |
| 9 | `supervisor` | Hecho | 8 unitarios + 4 de integración con procesos reales |
| 10 | `text_injector` | Lógica hecha | 4 tests unitarios + compila con `--features injector`; el pegado real queda pendiente |
| 11 | `pipeline` | Parcial: lógica hecha | 9 tests unitarios; cableado con audio pendiente |
| 12 | Hotkeys + comandos | Parcial: estado hecho | 4 tests unitarios; comandos Tauri pendientes |
| 13 | `PillWidget` | Hecho | 5 tests + capturas en claro y oscuro |
| 14 | `AddWordPopup` | Frontend hecho | 6 tests + capturas; comandos Tauri pendientes |
| 15 | Tray, autostart, modelo | Parcial: rutas hechas | 6 tests unitarios; el resto requiere Windows |

## Lo que hay que hacer en la máquina Windows

Ordenado por lo que desbloquea más:

1. **Task 1, step 3** — `npm run tauri dev` tiene que abrir la ventana. Es la
   primera confirmación de que el scaffold está bien; todo lo de UI depende de
   esto.
2. **Task 6** — implementar `Engine` con `whisper-rs`. El protocolo, el loop de
   IPC y el heartbeat ya están y testeados; lo único que falta es la llamada a
   whisper.cpp detrás del trait, con la feature `whisper`.
3. **Task 7, captura** — `AudioCapture` con `cpal` y el ejemplo `mic_probe`.
   En el callback va `Resampler16k`, no `resample_to_16k`.
4. **Tasks 11 a 15** — pipeline, hotkeys, píldora, popup de vocabulario y el
   servicio (bandeja, autostart, descarga del modelo).

También queda verificar a mano el pegado real del `text_injector`: compila, pero
compilar no es pegar bien en el Bloc de notas.

**El job de Windows del CI ya pasa** (desde el commit de los iconos): la app con
la feature `app` compila, clippy queda limpio y los tests corren. Eso confirma
que el scaffold de Tauri está bien armado; lo que sigue sin verificar es el
comportamiento en runtime, que es harina de otro costal.

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

### Task 9 — `supervisor`

**Qué se hizo.** `Sidecar` (spawn, send, try_recv, is_alive) y la política de
reinicio partida en dos piezas testeables: `Backoff` (0.5 s, 1 s, 2 s, 4 s,
techo de 5 s) y `Watchdog`.

**Decisiones que valen la pena.**

- *`Watchdog::tick` no mira el reloj*: recibe el tiempo transcurrido como
  parámetro. Así la política de reinicio se prueba entera sin dormir el test ni
  levantar procesos.
- *También reinicia a un sidecar vivo pero mudo.* Es el caso que importa: un
  cuelgue de CUDA deja el proceso existiendo, y un supervisor que sólo mire
  `try_wait` no lo detectaría nunca.
- *El primer reinicio de una racha es inmediato.* Hacer esperar después de un
  crash aislado sólo alarga el rato sin dictado; la espera creciente es para el
  sidecar que directamente no logra arrancar.
- *`impl Drop for Sidecar`* mata al proceso hijo. Sin eso, un sidecar
  reemplazado seguiría vivo ocupando VRAM.
- *`stderr` se hereda* en vez de descartarse como proponía el plan: los logs de
  whisper.cpp son lo único que dice si agarró CUDA o cayó a CPU.
- *`Backoff::next_delay`, no `next`*: clippy avisa —con razón— que un `next()`
  propio se confunde con `Iterator::next`.

**Pruebas ejecutadas.**

```
cargo test -p voice-type --no-default-features  → 42 unitarios + 4 de integración
cargo clippy ... -D warnings                    → sin warnings
```

Los 4 de integración levantan procesos de verdad (`tests/bin/echo_sidecar.rs`,
que habla el mismo protocolo sin modelo ni GPU): spawn e ida y vuelta por IPC,
detección de proceso muerto, un reemplazo que vuelve a atender pedidos, y un
binario inexistente que da error en vez de panic.

### Task 10 — `text_injector`

**Qué se hizo.** El trait `Clipboard`, la función `inject()` (guardar, poner el
texto, pegar, restaurar) y —detrás de la feature `app`— `SystemClipboard`
(arboard) y `paste_ctrl_v` (enigo).

**Decisiones.**

- *`inject` no duerme ni pega por sí misma.* Recibe el pegado como closure, así
  que el algoritmo —que es el que puede arruinarle el portapapeles al usuario—
  se testea entero sin tocar el portapapeles real. La espera de 120 ms que
  necesita la app destino para leer el portapapeles vive en `paste_ctrl_v`.
- *Un portapapeles vacío se restaura vacío*, no se deja la transcripción ahí. Es
  texto dictado por el usuario; no tiene por qué quedar disponible después.
- *Texto vacío es un no-op*: no pisa el portapapeles ni manda un Ctrl+V al vacío.

**Pruebas ejecutadas.**

```
cargo test -p voice-type --no-default-features text_injector  → 4 passed
cargo clippy ... -D warnings                                  → sin warnings
```

Uno de los tests mira el portapapeles *desde adentro* del pegado (estado
compartido con `Rc<RefCell<_>>`) para fijar el orden real de las operaciones, no
sólo que la closure se haya llamado.

**Feature `injector`.** `arboard` y `enigo` quedaron en su propia feature en vez
de colgar de `app`: no dependen de Tauri, así que su código real *sí* compila en
Linux. Eso saca al módulo que toca el portapapeles del usuario de la lista de
"código nunca compilado", y el CI lo revisa en el job de Linux con
`--features injector`.

**Pendiente de verificación manual (Windows 11).** Que el pegado funcione de
verdad contra otra aplicación, y el ejemplo `inject_probe` del Step 5. Compilar
no es lo mismo que pegar bien en el Bloc de notas.

### Task 7 — `audio` (parcial)

**Qué se hizo.** `downmix_to_mono`, `resample_to_16k` y —esto no estaba en el
plan— `Resampler16k`, el resample continuo para audio en vivo.

**Lo que encontró el test.** El test del plan esperaba que remuestrear 320
muestras de 32 kHz diera ~160 a 16 kHz. Dio 127. No es un bug del test: un
resampler recién creado arranca con el filtro sinc vacío y se come unas
`sinc_len/2` muestras de entrada. Como la función del plan crea un resampler
nuevo en cada llamada, usarla por cada bloque del micrófono metería ese hueco en
**cada frontera de bloque** — el audio saldría entrecortado, y con suerte se
notaría recién al escuchar una transcripción con palabras cortadas.

De ahí `Resampler16k`: mantiene un único resampler vivo entre bloques, así el
arranque se paga una sola vez. Acepta bloques de cualquier tamaño (cpal no
garantiza que midan siempre lo mismo) y guarda lo que no completa un chunk.
`resample_to_16k` queda para conversiones de una sola vez, con la advertencia
escrita en su documentación.

**Pruebas ejecutadas.**

```
cargo test -p voice-type --no-default-features audio  → 10 passed
cargo test -p voice-type --no-default-features        → 56 unitarios + 4 de integración
cargo clippy ... -D warnings                          → sin warnings
```

El test que importa es `streaming_resampler_does_not_lose_audio_between_blocks`:
diez bloques seguidos pierden sólo el arranque, una vez, y no una vez por
bloque.

**Pendiente (Windows, requiere micrófono).** `AudioCapture` con `cpal` y el
ejemplo `mic_probe` del Step 5. No compila en este entorno Linux (falta ALSA) y
su verificación es manual de todas formas. En el callback hay que usar
`Resampler16k`, no `resample_to_16k`.

### Task 11 — `pipeline` (parcial)

**Qué se hizo.** `Pipeline`: recibe cada segmento transcripto, le aplica las
correcciones del vocabulario, lo une con lo que ya se dictó y escribe en la
ventana enfocada.

**Lo que define el diseño.**

- *Se escribe sólo el pedazo nuevo, no la frase entera.* El texto se pega
  mientras la persona habla; reinyectar todo el acumulado en cada segmento lo
  duplicaría en pantalla. `on_result` calcula el delta contra el transcript.
- *`last_raw_segment()` devuelve lo que Whisper escuchó, sin corregir.* Es lo
  que le muestra el popup de vocabulario al usuario: tiene que poder tocar la
  palabra *mal entendida*, y en el texto ya corregido esa palabra no está.
- *Las anotaciones de "esto no es habla" se descartan.* Whisper marca música,
  silencio y ruido con `[BLANK_AUDIO]`, `(música)`, `[ Risas ]`. Pegar eso en el
  documento del usuario sería un desastre. El filtro sólo actúa cuando el
  segmento entero es la anotación: una frase real con paréntesis se escribe
  normal.
- *`finish()` limpia el transcript.* El próximo dictado puede ir a otra ventana;
  arrastrar el texto anterior capitalizaría mal y calcularía el delta contra
  algo que ya no está en pantalla.

**Pruebas ejecutadas.**

```
cargo test -p voice-type --no-default-features pipeline  → 9 passed
```

**Pendiente (Windows).** El cableado `AudioCapture` → `Segmenter` → `Sidecar` →
`on_result`, que depende de la captura pendiente de la Task 7.

### Task 12 — Hotkeys (parcial)

**Qué se hizo.** `DictationState` con `toggled()` e `is_dictating()`.

Apretar el atajo mientras se está transcribiendo **apaga**, no vuelve a
arrancar: para quien está dictando el toggle seguía prendido.

El test que más valor tiene es el que serializa las tres variantes y las compara
con el tipo `Status` de `src/lib/state.ts`. Si alguien renombra una variante en
Rust, la píldora se quedaría mostrando el estado viejo sin ningún error a la
vista; ese test es el que avisa.

**Pruebas ejecutadas.**

```
cargo test -p voice-type --no-default-features hotkeys  → 4 passed
```

**Pendiente (Windows).** `commands.rs` y el registro del atajo global, que
necesitan Tauri.

### Tasks 13 y 14 — Píldora y popup de vocabulario

**Qué se hizo.** `PillWidget.svelte` (translúcida, redondeada, arrastrable, con
las ondas animadas y el botón "+"), `AddWordPopup.svelte` (chips con las
palabras de la última frase, campo para la forma correcta) y `tokenize.ts`.

**Cómo se verificó el diseño sin Windows.** `npx vite preview` + capturas con
Chromium sobre un fondo a cuadros, en claro y en oscuro. Las llamadas a Tauri
fallan ahí —no hay backend— y los componentes lo manejan, así que se ve el
layout real. Lo que esa pasada no puede decir es si la translucidez y el
siempre-encima se comportan en Windows.

**Un bug que apareció justo por mirarlo.** La ventana es transparente y
siempre-encima. Con el alto fijo de 260 px, todo el espacio vacío debajo de la
píldora seguía siendo ventana: se comía los clicks de lo que hubiera abajo, o
sea un rectángulo de pantalla inutilizable flotando sobre el escritorio. Ahora
la ventana arranca con el alto de la píldora (64 px) y crece sólo mientras el
popup está abierto, con `core:window:allow-set-size` en las capabilities.

**Decisiones del frontend.**

- *Una sola ventana para la píldora y el popup.* Arrastrás la píldora y el
  popup la sigue; no quedan dos cosas separadas flotando.
- *`statusLabel` e `isAnimated` viven en `state.ts`, no en el componente.* Así
  se testean sin montar Svelte.
- *Las ondas siguen animadas en `Transcribing`.* El micrófono sigue grabando
  mientras el motor procesa el segmento anterior; congelarlas haría creer que
  dejó de escuchar.
- *`stripPunctuation` antes de guardar la palabra mal escuchada.* Si se
  guardara `yeison,` con la coma, el reemplazo no volvería a aplicarse nunca:
  el post-procesador busca por palabra completa.
- *El click en "+" corta la propagación*, si no también togglearía el dictado.
- *`prefers-reduced-motion`* apaga la animación de las ondas.

**Pruebas ejecutadas.**

```
npx vitest run     → 11 passed (5 de state, 6 de tokenize)
npx svelte-check   → 0 errores, 0 warnings
npm run build      → OK
Capturas en Chromium (claro y oscuro): píldora en reposo, píldora activa y popup
```

**Pendiente.** Los comandos `get_last_transcription` y `add_vocab_word` del
step 3 de la Task 14: la lógica ya está en `Pipeline`, falta exponerla desde
`commands.rs` en Windows. Y la pasada visual con `npm run tauri dev`.

### Task 15 — Descarga del modelo (parcial)

**Qué se hizo.** `model_path`, `partial_path`, `model_url` e
`is_valid_model_name`: todo lo que se puede decidir sin tocar la red.

**Dos cosas que valen más que las rutas en sí.**

- *La descarga va a un `.part` y recién al terminar se renombra.* Un corte de
  red a mitad de camino deja un `.part` incompleto en vez de un `.bin` truncado
  que, en el próximo arranque, parecería un modelo válido y haría fallar la
  carga con un error del motor nativo imposible de interpretar.
- *`is_valid_model_name` rechaza cualquier cosa que no sea `ggml-*.bin` sin
  rutas adentro.* `model_name` sale de `config.json`, que el usuario puede
  editar a mano: sin esa validación, un nombre como `../../algo.bin` escribiría
  la descarga fuera de la carpeta de modelos y apuntaría la URL a otro lugar
  del repositorio. Hay tests para `..`, barras, barras invertidas y rutas
  absolutas de Windows.

**Pruebas ejecutadas.**

```
cargo test -p voice-type --no-default-features model_download  → 6 passed
cargo test -p voice-type --no-default-features                 → 75 unitarios + 4 de integración
```

**Pendiente (Windows).** `ensure_model` con la descarga real, la bandeja, el
autostart, la instancia única y el logging con rotación diaria.

### Task 6 — Sidecar STT (preparada, falta el motor)

**Qué se hizo.** El sidecar quedó partido en dos: `stt-sidecar/src/lib.rs`
maneja el protocolo contra el trait `Engine`, y el motor real es una
implementación más de ese trait.

**Por qué.** `whisper-rs` necesita el CUDA Toolkit para compilar y una GPU para
correr. Con el manejo de mensajes pegado al motor, nada de eso se podía probar
en ningún CI. Con el trait, lo único que queda sin cobertura automática es la
llamada a whisper.cpp; todo el resto —qué contestar a cada mensaje, qué pasa si
el modelo no carga, que el `id` vuelva igual— se testea con un motor de mentira
en cualquier máquina.

`whisper-rs` va detrás de la feature `whisper`, apagada por defecto. El build de
producción es `cargo build -p stt-sidecar --features whisper --release`; sin
ella, el sidecar arranca igual y contesta un error que dice exactamente qué
falta.

**Detalle que importa.** Cuando falla un segmento, el error lleva el `id`
adentro del mensaje. Sin eso, del otro lado no hay forma de saber qué segmento
se perdió.

**Pruebas ejecutadas.**

```
cargo test -p stt-sidecar   → 6 passed
cargo clippy -p stt-sidecar --all-targets -- -D warnings  → sin warnings
```

Smoke test del binario, mandándole LoadModel, una línea basura, Transcribe y
Shutdown:

```
{"type":"Error","message":"sidecar compilado sin motor STT: falta la feature `whisper` (Task 6 del plan)"}
{"type":"Error","message":"segmento 9: sidecar compilado sin motor STT: falta la feature `whisper` (Task 6 del plan)"}
```

**Pendiente (Windows con CUDA).** Implementar `Engine` con `whisper-rs` y el
test de integración contra un modelo local.
