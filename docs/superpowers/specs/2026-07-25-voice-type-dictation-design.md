# Voice Type — Diseño (dictado por voz para Windows 11)

- **Fecha:** 2026-07-25
- **Estado:** Aprobado (brainstorming), pendiente plan de implementación
- **Autor:** Javier
- **Stack:** Rust + Tauri v2 (core) · Svelte (frontend) · whisper.cpp vía `whisper-rs` (STT)

---

## 1. Resumen

Herramienta de **dictado por voz** (voz → texto) para uso personal en Windows 11.
Corre en segundo plano como un servicio estable de bandeja. El usuario activa el
dictado con un atajo global (modo **toggle**: una vez inicia, otra vez detiene),
habla en **español**, y el texto transcrito se **inyecta en la aplicación que
tenga el foco** (Word, navegador, chat, etc.).

Incluye **vocabulario personalizado**: el reconocimiento está fijado a español
pero el usuario agrega, sobre la marcha, términos en inglés (jerga técnica,
nombres) que suele usar. La corrección se hace desde la propia interfaz con un
botón "+", sin editar archivos a mano.

La interfaz es **minimalista, estética tipo Apple** (translúcida, redondeada,
animaciones suaves), más limpia que el dictado nativo de Windows.

> **Nota de alcance:** Microsoft VibeVoice (la referencia inicial) es texto → voz
> (TTS). Este proyecto es lo contrario, voz → texto (STT/dictado), así que
> VibeVoice queda descartado.

## 2. Objetivos

- Dictado en español fijo, alta precisión, con GPU (RTX 4060, CUDA).
- Modo toggle por atajo global; texto insertado en la app enfocada.
- Vocabulario personalizado editable desde la UI (bias + reemplazos).
- Servicio **estable**: se recupera solo de crashes del motor; arranca al iniciar sesión.
- UI minimalista tipo Apple.
- Offline y privado (nada sale de la máquina).

## 3. No-objetivos (YAGNI)

- Sin TTS (texto → voz).
- Sin push-to-talk ni ventana de dictado propia (se eligió toggle + inyección).
- Sin multi-idioma con auto-detección (español fijo; el inglés entra vía vocabulario).
- Sin sincronización en la nube, cuentas ni telemetría.
- Sin soporte multiplataforma en v1 (solo Windows 11).
- Sin instalador distribuible pulido en v1 (uso personal; binario local basta).

## 4. Hardware objetivo

- GPU: NVIDIA RTX 4060 Laptop (8 GB VRAM), CUDA.
- CPU: Intel i7-13650HX. RAM: 40 GB.
- Modelo STT: `large-v3` cuantizado (q5/q8) en GPU; fallback CPU si CUDA no está disponible.

## 5. Stack tecnológico

| Capa | Elección | Motivo |
|---|---|---|
| Core / servicio | **Rust + Tauri v2** | Binario único, memory-safe, integración nativa Windows, estable. |
| Frontend | **Svelte** | Bundle chico, animaciones limpias, estética Apple con CSS. |
| STT | **`whisper-rs`** (whisper.cpp, feature `cuda`) | Autocontenido, estable en Windows, GPU. |
| Audio | **`cpal`** + resample a 16 kHz | Captura de micrófono multiplataforma. |
| VAD | Silero (`voice_activity_detector`) o `webrtc-vad` | Segmentar en pausas. |
| Inyección de texto | `tauri-plugin-clipboard-manager` + `enigo` (Ctrl+V) | Maneja tildes/Unicode; guarda/restaura portapapeles. |
| Atajos globales | `tauri-plugin-global-shortcut` | Toggle de dictado. |
| Autostart | `tauri-plugin-autostart` | Arranca al iniciar sesión (session interactiva, con GPU). |
| Instancia única | `tauri-plugin-single-instance` | Evita duplicados. |
| Config/estado | `serde` + `tauri-plugin-store` (JSON) | Persistencia simple. |
| Logs | `tracing` + file appender con rotación | Diagnóstico. |

## 6. Arquitectura

Dos procesos, patrón **supervisor + worker**, para que el servicio sea estable:

```
┌─────────────────────────────────────────────┐
│  Proceso principal (Tauri / Rust)            │
│  - UI (WebView2 + Svelte): pill, popup, tray │
│  - Hotkeys globales, autostart, single-inst. │
│  - VocabStore, PostProcessor, TextInjector   │
│  - Supervisor (spawn + watchdog del sidecar) │
└───────────────┬─────────────────────────────┘
                │ IPC: stdin/stdout JSON + heartbeat
┌───────────────┴─────────────────────────────┐
│  Sidecar STT (proceso aparte, Rust)          │
│  - whisper-rs (whisper.cpp + CUDA)           │
│  - Carga el modelo una vez, se mantiene warm │
└──────────────────────────────────────────────┘
```

**Por qué sidecar:** whisper.cpp es código nativo C++. Un segfault o un OOM de
VRAM en el mismo proceso tumbaría toda la app (incluida la UI). Aislado en un
proceso hijo, el supervisor detecta la caída (proceso muerto o heartbeat
perdido) y lo **reinicia solo**, sin afectar la UI ni perder el estado.

El audio se captura en el proceso principal y se envía al sidecar; el sidecar
devuelve segmentos transcritos. (Alternativa evaluada: capturar audio dentro del
sidecar; se prefiere capturar en el principal para que el sidecar sea sin estado
y reiniciable.)

## 7. Módulos (unidades aisladas)

Cada módulo tiene un propósito único, interfaz definida y se testea aislado.

### Proceso principal
- **`audio`** — captura de micrófono (`cpal`), resample a 16 kHz mono, entrega
  frames PCM. Dep: `cpal`.
- **`vad`** — recibe frames, detecta límites de habla/silencio, emite segmentos.
  Dep: crate VAD.
- **`supervisor`** — lanza el sidecar, monitorea heartbeat, reinicia en fallo,
  aplica backoff. Dep: `ipc`.
- **`ipc`** — protocolo JSON principal↔sidecar (serialización `serde`). Mensajes:
  `Transcribe{segment_id, pcm}`, `Result{segment_id, text}`, `Heartbeat`,
  `LoadModel{path, lang, hotwords}`, `Error{...}`.
- **`vocab_store`** — persistencia del vocabulario: lista de términos (para bias)
  + mapa `oído → correcto` (reemplazos). CRUD desde la UI. Dep: `store`.
- **`post_processor`** — aplica el mapa de reemplazos, normaliza espacios,
  capitalización y puntuación de unión entre segmentos. Puro/determinista.
- **`text_injector`** — guarda el portapapeles actual, coloca el texto, simula
  Ctrl+V (`enigo`), restaura el portapapeles. Dep: clipboard plugin + `enigo`.
- **`hotkeys`** — registra el atajo toggle, emite eventos start/stop.
- **`config`** — carga/guarda ajustes (atajo, modelo, dispositivo de audio).

### Sidecar
- **`stt`** — envuelve `whisper-rs`; carga modelo una vez; transcribe segmentos
  con `language="es"` + `hotwords`; responde por IPC; emite heartbeat.

### Frontend (Svelte)
- **`PillWidget`** — píldora flotante translúcida, arrastrable, con estado visual
  (idle / escuchando con onda animada / transcribiendo) y botón "+".
- **`AddWordPopup`** — muestra la última transcripción; el usuario toca la palabra
  mal escrita, escribe la correcta, guarda.
- **`TrayMenu`** — mostrar/ocultar píldora, ajustes, salir.

## 8. Flujos de datos

### 8.1 Dictado (toggle)
1. Atajo global (o clic en la píldora) → `hotkeys` emite `start`.
2. `audio` captura → `vad` segmenta en pausas.
3. Cada segmento → `ipc` → **sidecar `stt`** (español + hotwords) → texto.
4. `post_processor` aplica reemplazos y limpieza.
5. `text_injector` inserta en la app enfocada (portapapeles → Ctrl+V → restaura).
6. La píldora refleja el estado; el texto aparece **de a segmentos mientras
   hablás**, no al final.
7. Segundo atajo → `stop`, se procesa el último segmento pendiente.

### 8.2 Agregar / corregir palabra (botón "+")
1. Usuario toca "+" en la píldora → `AddWordPopup` con la última transcripción.
2. Selecciona el token mal transcrito y escribe la forma correcta.
3. `vocab_store` guarda: agrega el término a la lista de bias **y** una entrada
   `oído → correcto` en el mapa de reemplazos.
4. Se recargan los `hotwords` en el sidecar **en caliente** (mensaje IPC), sin
   reiniciar el modelo.

## 9. Vocabulario personalizado

Dos mecanismos combinados:

1. **Bias de reconocimiento** — los términos del usuario se pasan como `hotwords`
   / prompt inicial a whisper.cpp, aumentando la probabilidad de que los
   reconozca (útil para jerga en inglés dentro de habla en español).
2. **Corrección post-transcripción** — mapa determinista `oído → correcto` que
   arregla lo que el bias no alcanza (p. ej. Whisper escribe "yeison" → se
   reemplaza por `JSON`). Aplicado por `post_processor`.

Ambos se alimentan desde el mismo flujo del botón "+". Persistidos como JSON en
la carpeta de config; el usuario nunca los edita a mano.

## 10. Interfaz (estética Apple)

- **Píldora**: sin bordes, esquinas muy redondeadas, fondo translúcido
  (`backdrop-filter: blur`), sombra suave, tipografía system-ui. Transiciones
  tipo spring. Arrastrable; recuerda su posición.
- **Estados**: idle (ícono mic tenue), escuchando (onda/nivel animado),
  transcribiendo (indicador sutil). Sin ruido visual.
- **Popup "+"**: tarjeta redondeada, foco en la última frase, interacción de un
  toque para elegir palabra + campo de corrección.
- **Tray**: menú nativo mínimo.

## 11. Configuración y persistencia

- Carpeta de config de la app (`tauri::path` / AppConfig).
- Ajustes: atajo toggle, tamaño de modelo, dispositivo de micrófono, posición de
  la píldora.
- Vocabulario: `vocab.json` (términos + reemplazos).
- Modelo whisper: descargado al primer arranque a la carpeta de datos.

## 12. Resiliencia y manejo de errores

- **Sin micrófono / dispositivo cambiado**: aviso en la píldora, reintento al
  reconectar.
- **CUDA no disponible**: fallback automático a CPU (más lento) + aviso.
- **Crash / cuelgue del sidecar**: `supervisor` detecta (proceso muerto o
  heartbeat perdido) y reinicia con backoff; la UI sigue viva.
- **OOM de VRAM**: capturado como error del sidecar → reinicio; opción de bajar
  tamaño de modelo.
- **Fallo al pegar**: se restaura el portapapeles original igual.
- **Primer arranque sin modelo**: descarga con indicador de progreso.
- **Conflicto de atajo**: aviso y opción de reasignar.
- **Instancia única**: `single-instance` evita duplicados.

## 13. Autostart

- `tauri-plugin-autostart` registra el arranque al **iniciar sesión** (sesión
  interactiva, con acceso a GPU y escritorio).
- **No** se usa Windows Service (session 0 no tiene GPU ni UI: trampa conocida).

## 14. Testing

- **Unit**: `post_processor` (reemplazos, unión de segmentos, capitalización),
  `vocab_store` (CRUD, persistencia), `ipc` (serde round-trip), `vad` (límites en
  audio sintético).
- **Integración**: WAV de referencia en español → sidecar `stt` → assert de
  transcripción esperada; recarga de hotwords en caliente.
- **Manual / E2E**: dictado real en una app enfocada; flujo del botón "+";
  reinicio del sidecar simulando un crash.

## 15. Riesgos conocidos

- Construcción de `whisper-rs` con CUDA en Windows requiere toolchain correcto
  (CUDA toolkit); mitigado por ser máquina única y build local.
- Latencia percibida depende del tamaño de segmento del VAD; ajustable.
- Inyección por portapapeles pisa el portapapeles momentáneamente; mitigado con
  guardar/restaurar.

## 16. Fuera del spec (posible v2)

- Instalador/distribución firmada.
- Push-to-talk como modo alternativo.
- Comandos de voz (puntuación por voz, "nueva línea", etc.).
- Perfiles de vocabulario por aplicación.
