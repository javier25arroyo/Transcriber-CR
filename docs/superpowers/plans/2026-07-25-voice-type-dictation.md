# Voice Type — Plan de implementación

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Construir una app de dictado por voz (voz → texto) para Windows 11, en Rust + Tauri, que transcribe español con GPU y lo inyecta en la app enfocada, con vocabulario personalizado editable desde una UI minimalista.

**Architecture:** Dos procesos (supervisor + worker). El proceso principal Tauri maneja UI, audio, hotkeys, vocabulario, inyección de texto y supervisa un **sidecar** STT aislado (whisper.cpp vía `whisper-rs`) que se reinicia solo ante crashes/OOM. IPC por stdin/stdout con JSON + heartbeat.

**Tech Stack:** Rust (edición 2021), Tauri v2, Svelte, `whisper-rs` (feature `cuda`), `cpal`, `rubato`, `arboard`, `enigo`, `serde`/`serde_json`, `tracing`.

## Global Constraints

- Plataforma objetivo: **Windows 11 únicamente** (v1).
- Idioma de reconocimiento **fijo a español** (`"es"`); el inglés entra solo vía vocabulario personalizado.
- Modo de activación: **toggle** (un atajo inicia, otro detiene). No push-to-talk.
- Todo **offline**: sin red salvo la descarga inicial del modelo. Sin telemetría ni cuentas.
- STT corre en **proceso sidecar aislado**, nunca en el proceso de la UI.
- Rust edición 2021, `rustfmt` por defecto, `clippy` sin warnings en CI local.
- Commits frecuentes, uno por tarea como mínimo. Mensajes tipo `feat:`/`test:`/`chore:`.
- TDD donde la lógica es pura y determinista (`vocab_store`, `post_processor`, `ipc`, `config`, `vad`). Hardware/GPU/UI: test de integración con archivos de referencia + verificación manual documentada.

---

## Mapa de archivos

Workspace Cargo con dos binarios + frontend Svelte.

```
Voice Type/
├─ Cargo.toml                      # workspace
├─ src-tauri/
│  ├─ Cargo.toml                   # binario principal (app Tauri) + lib de módulos
│  ├─ tauri.conf.json
│  ├─ build.rs
│  └─ src/
│     ├─ main.rs                   # bootstrap Tauri, plugins, commands
│     ├─ lib.rs                    # re-exporta módulos para tests
│     ├─ config.rs                 # AppConfig load/save
│     ├─ vocab_store.rs            # términos + reemplazos, JSON
│     ├─ post_processor.rs         # reemplazos + limpieza (puro)
│     ├─ ipc.rs                    # tipos de mensajes principal<->sidecar (serde)
│     ├─ supervisor.rs             # spawn/watchdog/restart del sidecar
│     ├─ audio.rs                  # captura cpal + resample 16k mono
│     ├─ vad.rs                    # segmentación por energía/silencio (puro)
│     ├─ text_injector.rs          # portapapeles save/set + Ctrl+V + restore
│     ├─ hotkeys.rs                # registro atajo toggle
│     └─ commands.rs               # #[tauri::command] expuestos al frontend
├─ stt-sidecar/
│  ├─ Cargo.toml                   # binario sidecar
│  └─ src/main.rs                  # loop IPC: carga modelo, transcribe, heartbeat
├─ src/                            # frontend Svelte
│  ├─ main.ts
│  ├─ App.svelte
│  ├─ lib/
│  │  ├─ PillWidget.svelte
│  │  ├─ AddWordPopup.svelte
│  │  └─ state.ts                  # store de estado (idle/listening/transcribing)
│  └─ app.css
├─ index.html
├─ package.json
└─ vite.config.ts
```

Los módulos de `src-tauri/src` se exponen vía `lib.rs` para poder testearlos con `cargo test` sin levantar la UI. `ipc.rs` es compartido: el sidecar depende de `src-tauri` como librería (o se duplica el módulo en un crate `shared`; ver Task 5).

---

## Task 1: Scaffold del proyecto (Tauri v2 + Svelte)

**Files:**
- Create: `package.json`, `vite.config.ts`, `index.html`, `src/main.ts`, `src/App.svelte`, `src/app.css`
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/build.rs`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`
- Create: `Cargo.toml` (workspace), `.gitignore`

**Interfaces:**
- Produces: proyecto que compila y abre una ventana; base para todos los módulos.

- [x] **Step 1: Inicializar git y scaffold Tauri**

Requisitos previos (instalar una sola vez): Rust (`rustup`), Node LTS, y para el sidecar CUDA: CUDA Toolkit 12.x. Verificar: `rustc --version`, `node --version`, `nvcc --version`.

```bash
cd "D:/GitHub/Voice Type"
git init
npm create tauri-app@latest . -- --template svelte-ts --manager npm --yes
```

Si el generador se niega por carpeta no vacía (existe `docs/`), crear en subcarpeta temporal y mover, o usar `--force`. Ajustar `src-tauri/tauri.conf.json` → `productName: "Voice Type"`, `identifier: "com.javier.voicetype"`.

- [x] **Step 2: Convertir a workspace y verificar build**

Editar `Cargo.toml` raíz:

```toml
[workspace]
members = ["src-tauri", "stt-sidecar"]
resolver = "2"
```

- [x] **Step 3: Ejecutar en dev para confirmar que abre**

Run: `pnpm tauri dev`
Expected: abre una ventana de Tauri sin errores. Cerrar.

> **Verificado en Windows 11 (2026-07-27).** Encontró un bug real: el crate
> tiene dos `[[bin]]` (`voice-type` y `echo-sidecar`, el sidecar de mentira de
> los tests de integración), y `cargo run` — que `tauri dev` invoca sin
> `--bin` — no podía elegir entre los dos (`error: cargo run could not
> determine which binary to run`). Arreglado con `default-run = "voice-type"`
> en `[package]` de `src-tauri/Cargo.toml`, tal como sugería el propio mensaje
> de error. Con eso, compiló y la píldora abrió: translúcida, "Dictado
> apagado", botón "+", confirmado por captura de pantalla. Ver
> [docs/progress.md](../../progress.md).

- [x] **Step 4: Retirar las guardas temporales del CI**

`Cargo.toml` y `package.json` ya existen. En `.github/workflows/ci.yml`,
quitar los `if: hashFiles('Cargo.toml') != ''` y
`if: hashFiles('package.json') != ''` de cada step, y volver `cache: npm` fijo
(sin condicional) en el step "Install Node.js" (se agregaron para que el CI no
quedara en rojo antes de que hubiera código — ver
`docs/superpowers/specs/2026-07-26-sdlc-docs-design.md` §7). A partir de este
commit, la ausencia de esos manifiestos debe volver a hacer fallar el CI.

- [x] **Step 5: Commit**

```bash
git add -A
git commit -m "chore: scaffold Tauri v2 + Svelte project"
git add .github/workflows/ci.yml
git commit -m "ci: remove empty-repo guards now that manifests exist"
```

---

## Task 2: Módulo `vocab_store` (vocabulario personalizado)

**Files:**
- Create: `src-tauri/src/vocab_store.rs`
- Modify: `src-tauri/src/lib.rs` (agregar `pub mod vocab_store;`)
- Test: en el mismo archivo con `#[cfg(test)] mod tests`.

**Interfaces:**
- Produces:
  - `struct Vocab { terms: Vec<String>, replacements: Vec<Replacement> }`
  - `struct Replacement { heard: String, correct: String }`
  - `impl Vocab`:
    - `fn load(path: &Path) -> Vocab` (si no existe, `Vocab::default()`)
    - `fn save(&self, path: &Path) -> io::Result<()>`
    - `fn add(&mut self, heard: &str, correct: &str)` (agrega término `correct` a `terms` sin duplicar, y una `Replacement`)
    - `fn initial_prompt(&self) -> String` (concatena términos para sesgo de whisper)

- [x] **Step 1: Escribir el test que falla**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_inserts_term_and_replacement_without_duplicates() {
        let mut v = Vocab::default();
        v.add("yeison", "JSON");
        v.add("yeison", "JSON"); // duplicado
        assert_eq!(v.terms, vec!["JSON".to_string()]);
        assert_eq!(v.replacements.len(), 1);
        assert_eq!(v.replacements[0].heard, "yeison");
        assert_eq!(v.replacements[0].correct, "JSON");
    }

    #[test]
    fn initial_prompt_lists_terms() {
        let mut v = Vocab::default();
        v.add("guit jab", "GitHub");
        v.add("rasp", "Rust");
        let p = v.initial_prompt();
        assert!(p.contains("GitHub"));
        assert!(p.contains("Rust"));
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vocab.json");
        let mut v = Vocab::default();
        v.add("yeison", "JSON");
        v.save(&path).unwrap();
        let loaded = Vocab::load(&path);
        assert_eq!(loaded.terms, v.terms);
        assert_eq!(loaded.replacements.len(), 1);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let v = Vocab::load(Path::new("no/existe/vocab.json"));
        assert!(v.terms.is_empty());
    }
}
```

Agregar `tempfile` a `[dev-dependencies]` en `src-tauri/Cargo.toml`.

- [x] **Step 2: Correr el test para verificar que falla**

Run: `cargo test -p voice-type vocab_store`
Expected: FALLA (no compila: `Vocab` no existe).

- [x] **Step 3: Implementación mínima**

```rust
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Vocab {
    pub terms: Vec<String>,
    pub replacements: Vec<Replacement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replacement {
    pub heard: String,
    pub correct: String,
}

impl Vocab {
    pub fn load(path: &Path) -> Vocab {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)
    }

    pub fn add(&mut self, heard: &str, correct: &str) {
        if !self.terms.iter().any(|t| t == correct) {
            self.terms.push(correct.to_string());
        }
        self.replacements.push(Replacement {
            heard: heard.to_string(),
            correct: correct.to_string(),
        });
    }

    pub fn initial_prompt(&self) -> String {
        // whisper.cpp sesga con initial_prompt (no hay param hotwords formal).
        self.terms.join(", ")
    }
}
```

- [x] **Step 4: Correr los tests**

Run: `cargo test -p voice-type vocab_store`
Expected: PASA.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/vocab_store.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat: vocab_store with terms, replacements and JSON persistence"
```

---

## Task 3: Módulo `post_processor` (reemplazos + limpieza)

**Files:**
- Create: `src-tauri/src/post_processor.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: en el mismo archivo.

**Interfaces:**
- Consumes: `Vocab` / `Replacement` de Task 2.
- Produces:
  - `fn apply(raw: &str, replacements: &[Replacement]) -> String`
  - `fn join_segments(prev: &str, next: &str) -> String` (une con espacio y capitaliza inicio de oración)

- [x] **Step 1: Escribir el test que falla**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocab_store::Replacement;

    fn r(h: &str, c: &str) -> Replacement {
        Replacement { heard: h.into(), correct: c.into() }
    }

    #[test]
    fn apply_replaces_case_insensitive_whole_words() {
        let reps = vec![r("yeison", "JSON")];
        assert_eq!(apply("mandale un Yeison", &reps), "mandale un JSON");
    }

    #[test]
    fn apply_does_not_replace_inside_words() {
        let reps = vec![r("son", "SUN")];
        assert_eq!(apply("la canción", &reps), "la canción");
    }

    #[test]
    fn join_segments_adds_space_and_capitalizes_sentence_start() {
        assert_eq!(join_segments("hola.", "que tal"), "hola. Que tal");
        assert_eq!(join_segments("hola", "mundo"), "hola mundo");
    }
}
```

- [x] **Step 2: Correr el test para verificar que falla**

Run: `cargo test -p voice-type post_processor`
Expected: FALLA (funciones no definidas).

- [x] **Step 3: Implementación mínima**

```rust
use crate::vocab_store::Replacement;

pub fn apply(raw: &str, replacements: &[Replacement]) -> String {
    let mut out = raw.to_string();
    for rep in replacements {
        out = replace_whole_word_ci(&out, &rep.heard, &rep.correct);
    }
    out
}

fn replace_whole_word_ci(text: &str, heard: &str, correct: &str) -> String {
    // Reemplazo por palabra completa, case-insensitive, respetando límites unicode.
    let mut result = String::with_capacity(text.len());
    let lower_text = text.to_lowercase();
    let lower_heard = heard.to_lowercase();
    let mut idx = 0;
    while let Some(pos) = lower_text[idx..].find(&lower_heard) {
        let start = idx + pos;
        let end = start + lower_heard.len();
        let before_ok = start == 0
            || !text[..start].chars().next_back().map_or(false, |c| c.is_alphanumeric());
        let after_ok = end == text.len()
            || !text[end..].chars().next().map_or(false, |c| c.is_alphanumeric());
        if before_ok && after_ok {
            result.push_str(&text[idx..start]);
            result.push_str(correct);
        } else {
            result.push_str(&text[idx..end]);
        }
        idx = end;
    }
    result.push_str(&text[idx..]);
    result
}

pub fn join_segments(prev: &str, next: &str) -> String {
    let prev_t = prev.trim_end();
    let next_t = next.trim_start();
    if prev_t.is_empty() {
        return next_t.to_string();
    }
    let capitalize = prev_t.ends_with(['.', '!', '?']);
    let next_final = if capitalize { capitalize_first(next_t) } else { next_t.to_string() };
    format!("{prev_t} {next_final}")
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
```

- [x] **Step 4: Correr los tests**

Run: `cargo test -p voice-type post_processor`
Expected: PASA.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/post_processor.rs src-tauri/src/lib.rs
git commit -m "feat: post_processor with word-boundary replacements and segment joining"
```

---

## Task 4: Módulo `ipc` (protocolo principal ↔ sidecar)

**Files:**
- Create: `src-tauri/src/ipc.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: en el mismo archivo.

**Interfaces:**
- Produces (mensajes serializables a JSON, uno por línea):
  - `enum ToSidecar { LoadModel { model_path: String, language: String, initial_prompt: String }, Transcribe { id: u64, pcm16k: Vec<f32> }, Shutdown }`
  - `enum FromSidecar { Ready, Result { id: u64, text: String }, Heartbeat, Error { message: String } }`
  - `fn encode<T: Serialize>(msg: &T) -> String` (JSON + `\n`)
  - `fn decode<T: DeserializeOwned>(line: &str) -> Result<T, serde_json::Error>`

- [x] **Step 1: Escribir el test que falla**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_sidecar_roundtrips() {
        let msg = ToSidecar::Transcribe { id: 7, pcm16k: vec![0.1, -0.2] };
        let line = encode(&msg);
        assert!(line.ends_with('\n'));
        let back: ToSidecar = decode(line.trim()).unwrap();
        match back {
            ToSidecar::Transcribe { id, pcm16k } => {
                assert_eq!(id, 7);
                assert_eq!(pcm16k.len(), 2);
            }
            _ => panic!("variante incorrecta"),
        }
    }

    #[test]
    fn from_sidecar_result_roundtrips() {
        let msg = FromSidecar::Result { id: 3, text: "hola".into() };
        let back: FromSidecar = decode(encode(&msg).trim()).unwrap();
        assert!(matches!(back, FromSidecar::Result { id: 3, .. }));
    }
}
```

- [x] **Step 2: Correr el test para verificar que falla**

Run: `cargo test -p voice-type ipc`
Expected: FALLA.

- [x] **Step 3: Implementación mínima**

```rust
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToSidecar {
    LoadModel { model_path: String, language: String, initial_prompt: String },
    Transcribe { id: u64, pcm16k: Vec<f32> },
    Shutdown,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FromSidecar {
    Ready,
    Result { id: u64, text: String },
    Heartbeat,
    Error { message: String },
}

pub fn encode<T: Serialize>(msg: &T) -> String {
    let mut s = serde_json::to_string(msg).expect("serialize");
    s.push('\n');
    s
}

pub fn decode<T: DeserializeOwned>(line: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(line)
}
```

- [x] **Step 4: Correr los tests**

Run: `cargo test -p voice-type ipc`
Expected: PASA.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/ipc.rs src-tauri/src/lib.rs
git commit -m "feat: ipc message protocol with JSON line encoding"
```

---

## Task 5: Módulo `config` (ajustes) y crate compartido

**Files:**
- Create: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `stt-sidecar/Cargo.toml` (depender de `voice-type` como lib para reusar `ipc`)
- Test: en `config.rs`.

**Interfaces:**
- Produces:
  - `struct AppConfig { toggle_shortcut: String, model_name: String, mic_device: Option<String>, pill_pos: Option<(i32,i32)> }` con `Default` (`toggle_shortcut = "Ctrl+Alt+Space"`, `model_name = "ggml-large-v3-q5_0.bin"`).
  - `fn load(path: &Path) -> AppConfig`
  - `fn save(&self, path: &Path) -> io::Result<()>`

- [x] **Step 1: Escribir el test que falla**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_toggle_shortcut() {
        let c = AppConfig::default();
        assert_eq!(c.toggle_shortcut, "Ctrl+Alt+Space");
    }

    #[test]
    fn save_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let mut c = AppConfig::default();
        c.mic_device = Some("Mic USB".into());
        c.save(&path).unwrap();
        assert_eq!(AppConfig::load(&path).mic_device.as_deref(), Some("Mic USB"));
    }
}
```

- [x] **Step 2: Correr el test para verificar que falla**

Run: `cargo test -p voice-type config`
Expected: FALLA.

- [x] **Step 3: Implementación mínima**

```rust
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub toggle_shortcut: String,
    pub model_name: String,
    pub mic_device: Option<String>,
    pub pill_pos: Option<(i32, i32)>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            toggle_shortcut: "Ctrl+Alt+Space".into(),
            model_name: "ggml-large-v3-q5_0.bin".into(),
            mic_device: None,
            pill_pos: None,
        }
    }
}

impl AppConfig {
    pub fn load(path: &Path) -> AppConfig {
        std::fs::read_to_string(path).ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(p) = path.parent() { std::fs::create_dir_all(p)?; }
        std::fs::write(path, serde_json::to_string_pretty(self)?)
    }
}
```

- [x] **Step 4: Configurar el sidecar para reusar `ipc`**

En `stt-sidecar/Cargo.toml` agregar dependencia al lib del proyecto principal (nombre del paquete `voice-type`):

```toml
[dependencies]
voice-type = { path = "../src-tauri" }
serde_json = "1"
```

Asegurar en `src-tauri/Cargo.toml` que existe target lib:

```toml
[lib]
name = "voice_type"
path = "src/lib.rs"
```

- [x] **Step 5: Correr tests y commit**

Run: `cargo test -p voice-type config`
Expected: PASA.

```bash
git add src-tauri/src/config.rs src-tauri/src/lib.rs src-tauri/Cargo.toml stt-sidecar/Cargo.toml
git commit -m "feat: AppConfig persistence and shared lib for sidecar"
```

---

## Task 6: Sidecar STT (`whisper-rs`, GPU)

**Files:**
- Modify: `stt-sidecar/Cargo.toml` (agregar `whisper-rs`)
- Create/Modify: `stt-sidecar/src/main.rs`
- Test: `stt-sidecar/tests/transcribe_wav.rs` (integración, `#[ignore]` si no hay modelo/GPU en CI)

**Interfaces:**
- Consumes: `voice_type::ipc::{ToSidecar, FromSidecar, encode, decode}`.
- Produces: binario que lee `ToSidecar` de stdin (una línea JSON por mensaje) y escribe `FromSidecar` a stdout. Emite `Ready` al cargar modelo, `Result` por cada `Transcribe`, `Heartbeat` cada 2s, `Error` ante fallo.

> **Preparada.** El sidecar ya está partido en dos: el manejo del protocolo
> (`stt-sidecar/src/lib.rs`) contra el trait `Engine`, testeado con un motor de
> mentira y sin GPU, y el motor real, que es lo único que falta. `whisper-rs`
> va detrás de la feature `whisper` (apagada por defecto) para que el crate
> siga compilando en el CI sin CUDA Toolkit. Lo que queda de esta task es
> implementar `Engine` con `whisper-rs` y el test de integración con un modelo
> local.

- [ ] **Step 1: Agregar dependencia y helper de transcripción**

`stt-sidecar/Cargo.toml`:

```toml
[dependencies]
voice-type = { path = "../src-tauri" }
whisper-rs = { version = "0.13", features = ["cuda"] }
serde_json = "1"
```

- [ ] **Step 2: Escribir el test de integración que falla**

Colocar `stt-sidecar/tests/fixtures/hola.wav` (audio 16k mono, voz en español diciendo "hola mundo") y `models/ggml-base-q5_0.bin` (modelo chico para test rápido).

```rust
// stt-sidecar/tests/transcribe_wav.rs
use stt_sidecar::transcribe_pcm; // función pública extraída del main

#[test]
#[ignore] // requiere modelo local; correr con: cargo test -p stt-sidecar -- --ignored
fn transcribes_reference_wav() {
    let pcm = load_wav_16k_mono("tests/fixtures/hola.wav");
    let text = transcribe_pcm(
        "models/ggml-base-q5_0.bin", "es", "", &pcm,
    ).unwrap();
    assert!(text.to_lowercase().contains("hola"));
}

fn load_wav_16k_mono(path: &str) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path).unwrap();
    reader.samples::<i16>().map(|s| s.unwrap() as f32 / 32768.0).collect()
}
```

Extraer la lógica a `stt-sidecar/src/lib.rs` con `pub fn transcribe_pcm(...)` para poder testearla; `main.rs` solo hace el loop IPC. Agregar `hound` a `[dev-dependencies]`.

- [ ] **Step 3: Correr el test para verificar que falla**

Run: `cargo test -p stt-sidecar -- --ignored`
Expected: FALLA (función no existe / no compila).

- [ ] **Step 4: Implementar `transcribe_pcm` y el loop IPC**

`stt-sidecar/src/lib.rs`:

```rust
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

pub fn transcribe_pcm(
    model_path: &str,
    language: &str,
    initial_prompt: &str,
    pcm16k: &[f32],
) -> Result<String, String> {
    let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
        .map_err(|e| format!("load model: {e}"))?;
    let mut state = ctx.create_state().map_err(|e| e.to_string())?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some(language));
    if !initial_prompt.is_empty() {
        params.set_initial_prompt(initial_prompt);
    }
    params.set_print_progress(false);
    state.full(params, pcm16k).map_err(|e| e.to_string())?;
    let n = state.full_n_segments().map_err(|e| e.to_string())?;
    let mut out = String::new();
    for i in 0..n {
        out.push_str(&state.full_get_segment_text(i).map_err(|e| e.to_string())?);
    }
    Ok(out.trim().to_string())
}
```

`stt-sidecar/src/main.rs` (loop IPC; el contexto se carga una vez con `LoadModel` y se mantiene warm):

```rust
use std::io::{BufRead, Write};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use voice_type::ipc::{decode, encode, FromSidecar, ToSidecar};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

fn main() {
    let stdout = std::io::stdout();
    let (hb_tx, hb_rx) = mpsc::channel::<()>();
    // Heartbeat en hilo aparte.
    {
        let out = stdout.clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(2));
            if hb_rx.try_recv().is_ok() { break; }
            let _ = out.lock().write_all(encode(&FromSidecar::Heartbeat).as_bytes());
        });
    }

    let mut ctx: Option<WhisperContext> = None;
    let mut lang = String::from("es");
    let mut prompt = String::new();
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let line = match line { Ok(l) => l, Err(_) => break };
        let msg: ToSidecar = match decode(&line) { Ok(m) => m, Err(_) => continue };
        match msg {
            ToSidecar::LoadModel { model_path, language, initial_prompt } => {
                lang = language;
                prompt = initial_prompt;
                match WhisperContext::new_with_params(&model_path, WhisperContextParameters::default()) {
                    Ok(c) => { ctx = Some(c); emit(&stdout, &FromSidecar::Ready); }
                    Err(e) => emit(&stdout, &FromSidecar::Error { message: format!("load: {e}") }),
                }
            }
            ToSidecar::Transcribe { id, pcm16k } => {
                match transcribe_with(&ctx, &lang, &prompt, &pcm16k) {
                    Ok(text) => emit(&stdout, &FromSidecar::Result { id, text }),
                    Err(message) => emit(&stdout, &FromSidecar::Error { message }),
                }
            }
            ToSidecar::Shutdown => { let _ = hb_tx.send(()); break; }
        }
    }
}

fn emit(out: &std::io::Stdout, msg: &FromSidecar) {
    let _ = out.lock().write_all(encode(msg).as_bytes());
    let _ = out.lock().flush();
}

fn transcribe_with(ctx: &Option<WhisperContext>, lang: &str, prompt: &str, pcm: &[f32]) -> Result<String, String> {
    let ctx = ctx.as_ref().ok_or("modelo no cargado")?;
    let mut state = ctx.create_state().map_err(|e| e.to_string())?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    params.set_language(Some(lang));
    if !prompt.is_empty() { params.set_initial_prompt(prompt); }
    params.set_print_progress(false);
    state.full(params, pcm).map_err(|e| e.to_string())?;
    let n = state.full_n_segments().map_err(|e| e.to_string())?;
    let mut out = String::new();
    for i in 0..n { out.push_str(&state.full_get_segment_text(i).map_err(|e| e.to_string())?); }
    Ok(out.trim().to_string())
}
```

- [ ] **Step 5: Correr el test (con modelo local) y verificar que pasa**

Run: `cargo test -p stt-sidecar -- --ignored`
Expected: PASA (transcribe "hola ..."). Verifica que `whisper-rs` con CUDA compila y usa GPU (log de whisper indica backend CUDA).

- [ ] **Step 6: Commit**

```bash
git add stt-sidecar/
git commit -m "feat: STT sidecar with whisper-rs CUDA and IPC loop"
```

---

## Task 7: Módulo `audio` (captura + resample a 16k)

**Files:**
- Create: `src-tauri/src/audio.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml` (`cpal`, `rubato`)
- Test: unit para el resample/downmix (puro); captura real = verificación manual.

**Interfaces:**
- Produces:
  - `fn downmix_to_mono(interleaved: &[f32], channels: usize) -> Vec<f32>`
  - `fn resample_to_16k(mono: &[f32], src_rate: u32) -> Vec<f32>`
  - `struct AudioCapture` con `fn start(device: Option<&str>, sink: Sender<Vec<f32>>) -> Result<Stream>` (entrega bloques mono 16k). La captura viva se verifica a mano.

- [x] **Step 1: Escribir el test que falla (lógica pura)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_averages_stereo() {
        let stereo = [1.0, 0.0, 0.0, 1.0]; // L,R,L,R
        assert_eq!(downmix_to_mono(&stereo, 2), vec![0.5, 0.5]);
    }

    #[test]
    fn resample_halves_length_from_32k_to_16k() {
        let src: Vec<f32> = (0..320).map(|i| i as f32).collect();
        let out = resample_to_16k(&src, 32_000);
        // ~mitad de muestras (tolerancia por filtro).
        assert!((out.len() as i32 - 160).abs() <= 8);
    }
}
```

- [x] **Step 2: Correr el test para verificar que falla**

Run: `cargo test -p voice-type audio`
Expected: FALLA.

- [x] **Step 3: Implementación mínima**

```rust
use rubato::{Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction};

pub fn downmix_to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 { return interleaved.to_vec(); }
    interleaved
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

pub fn resample_to_16k(mono: &[f32], src_rate: u32) -> Vec<f32> {
    if src_rate == 16_000 { return mono.to_vec(); }
    let params = SincInterpolationParameters {
        sinc_len: 128, f_cutoff: 0.95, oversampling_factor: 128,
        interpolation: SincInterpolationType::Linear, window: WindowFunction::BlackmanHarris2,
    };
    let ratio = 16_000f64 / src_rate as f64;
    let mut resampler = SincFixedIn::<f32>::new(ratio, 2.0, params, mono.len(), 1).unwrap();
    let out = resampler.process(&[mono.to_vec()], None).unwrap();
    out.into_iter().next().unwrap_or_default()
}
```

`AudioCapture::start` con `cpal`: abrir el device de entrada por defecto (o por nombre), en el callback hacer `downmix_to_mono` + acumular hasta ~30 ms, `resample_to_16k`, y enviar por el `Sender`. (El stream vivo no se testea en unit; ver Step 5.)

> **Completo, verificado en Windows 11 (2026-07-27).** La conversión pura
> (`downmix_to_mono`, `resample_to_16k`, `Resampler16k`) ya estaba. Se agregó
> `CaptureConverter` — encadena downmix y `Resampler16k`, pura y testeada, es
> lo que un callback de captura necesita hacer con cada bloque — y sobre eso,
> `AudioCapture` con `cpal 0.17` (feature `capture`, incluida en `app`).
> `cpal 0.18.1` (la última al momento de escribir esto) no se pudo usar: trae
> dos versiones incompatibles de `windows-core` en su propio árbol de
> dependencias y no compila; `0.17.3` no tiene ese problema.
>
> `capture` no se prueba en el job de Linux del CI (necesitaría ALSA); se
> compila y prueba de verdad sólo en el job de Windows, dentro de `app`.

- [x] **Step 4: Correr tests unit**

Run: `cargo test -p voice-type audio`
Expected: PASA. (77 tests en `voice-type`, incluyendo `CaptureConverter`.)

- [x] **Step 5: Verificación manual de captura**

Agregar un ejemplo `src-tauri/examples/mic_probe.rs` que capture 3s e imprima nº de muestras y RMS. Run: `cargo run -p voice-type --example mic_probe --features capture`.

> **Verificado parcialmente.** El pipeline funciona de punta a punta:
> dispositivo encontrado, stream armado, 47498 muestras en ~3 s (48000
> esperadas a 16 kHz), sin errores. Lo que un agente no puede verificar es "RMS
> sube al hablar" — no hay forma de producir sonido real. Falta que una persona
> corra el ejemplo hablando frente al micrófono y confirme que el RMS sube.

- [x] **Step 6: Commit**

```bash
git add src-tauri/src/audio.rs src-tauri/src/lib.rs src-tauri/Cargo.toml src-tauri/examples/mic_probe.rs
git commit -m "feat: audio capture with mono downmix and 16k resample"
```

---

## Task 8: Módulo `vad` (segmentación por silencio)

**Files:**
- Create: `src-tauri/src/vad.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: en el mismo archivo (audio sintético).

**Interfaces:**
- Produces:
  - `struct Segmenter { /* estado interno */ }`
  - `impl Segmenter`: `fn new(silence_ms: u32, sample_rate: u32) -> Self`; `fn push(&mut self, frame: &[f32]) -> Option<Vec<f32>>` (devuelve un segmento cerrado cuando detecta silencio suficiente tras habla); `fn flush(&mut self) -> Option<Vec<f32>>` (cierra al parar el toggle).
- Umbral de energía (RMS) configurable; por defecto 0.01.

- [x] **Step 1: Escribir el test que falla**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn loud(n: usize) -> Vec<f32> { vec![0.3; n] }
    fn silence(n: usize) -> Vec<f32> { vec![0.0; n] }

    #[test]
    fn emits_segment_after_silence_following_speech() {
        // 16k, 300ms de silencio para cerrar.
        let mut seg = Segmenter::new(300, 16_000);
        assert!(seg.push(&loud(1600)).is_none());          // 100ms habla
        // suficiente silencio para cerrar (>=300ms):
        let out = seg.push(&silence(6400));                 // 400ms silencio
        assert!(out.is_some());
        assert!(out.unwrap().len() >= 1600);
    }

    #[test]
    fn flush_returns_pending_speech() {
        let mut seg = Segmenter::new(300, 16_000);
        seg.push(&loud(1600));
        assert!(seg.flush().is_some());
    }

    #[test]
    fn pure_silence_emits_nothing() {
        let mut seg = Segmenter::new(300, 16_000);
        assert!(seg.push(&silence(8000)).is_none());
        assert!(seg.flush().is_none());
    }
}
```

- [x] **Step 2: Correr el test para verificar que falla**

Run: `cargo test -p voice-type vad`
Expected: FALLA.

- [x] **Step 3: Implementación mínima**

```rust
pub struct Segmenter {
    threshold: f32,
    silence_needed: usize, // muestras de silencio para cerrar
    buffer: Vec<f32>,
    trailing_silence: usize,
    has_speech: bool,
}

impl Segmenter {
    pub fn new(silence_ms: u32, sample_rate: u32) -> Self {
        Self {
            threshold: 0.01,
            silence_needed: (silence_ms as usize * sample_rate as usize) / 1000,
            buffer: Vec::new(),
            trailing_silence: 0,
            has_speech: false,
        }
    }

    pub fn push(&mut self, frame: &[f32]) -> Option<Vec<f32>> {
        let rms = (frame.iter().map(|s| s * s).sum::<f32>() / frame.len().max(1) as f32).sqrt();
        let is_speech = rms >= self.threshold;
        if is_speech {
            self.has_speech = true;
            self.trailing_silence = 0;
            self.buffer.extend_from_slice(frame);
        } else if self.has_speech {
            self.trailing_silence += frame.len();
            self.buffer.extend_from_slice(frame);
            if self.trailing_silence >= self.silence_needed {
                return Some(self.take());
            }
        }
        None
    }

    pub fn flush(&mut self) -> Option<Vec<f32>> {
        if self.has_speech { Some(self.take()) } else { None }
    }

    fn take(&mut self) -> Vec<f32> {
        let out = std::mem::take(&mut self.buffer);
        self.trailing_silence = 0;
        self.has_speech = false;
        out
    }
}
```

- [x] **Step 4: Correr los tests**

Run: `cargo test -p voice-type vad`
Expected: PASA.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/vad.rs src-tauri/src/lib.rs
git commit -m "feat: energy-based silence segmenter for VAD"
```

---

## Task 9: Módulo `supervisor` (spawn + watchdog + restart)

**Files:**
- Create: `src-tauri/src/supervisor.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: en el mismo archivo, usando un binario "fake sidecar" (script) para no requerir GPU.

**Interfaces:**
- Consumes: `ipc::{ToSidecar, FromSidecar, encode, decode}`.
- Produces:
  - `struct Sidecar { /* child + stdin + rx de FromSidecar */ }`
  - `impl Sidecar`: `fn spawn(exe: &Path) -> io::Result<Sidecar>`; `fn send(&mut self, msg: &ToSidecar)`; `fn try_recv(&self) -> Option<FromSidecar>`; `fn is_alive(&mut self) -> bool`.
  - `struct Supervisor` con política: si `is_alive()==false` o no llega `Heartbeat` en 6s → re-spawn con backoff (0.5s, 1s, 2s, máx 5s) y re-`LoadModel`.

- [x] **Step 1: Escribir el test que falla**

Fake sidecar de test: `src-tauri/tests/bin/echo_sidecar.rs` que responde `Ready` a `LoadModel` y `Result{id,text:"ok"}` a `Transcribe`.

```rust
// en supervisor.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::{FromSidecar, ToSidecar};

    #[test]
    fn spawn_send_receive_with_fake_sidecar() {
        let exe = test_fake_sidecar_path(); // compilado por build de tests
        let mut s = Sidecar::spawn(&exe).unwrap();
        s.send(&ToSidecar::LoadModel { model_path: "x".into(), language: "es".into(), initial_prompt: "".into() });
        assert!(wait_for(&s, |m| matches!(m, FromSidecar::Ready)));
        s.send(&ToSidecar::Transcribe { id: 1, pcm16k: vec![0.0; 10] });
        assert!(wait_for(&s, |m| matches!(m, FromSidecar::Result { id: 1, .. })));
    }

    #[test]
    fn detects_dead_process() {
        let exe = test_fake_sidecar_path();
        let mut s = Sidecar::spawn(&exe).unwrap();
        s.send(&ToSidecar::Shutdown);
        std::thread::sleep(std::time::Duration::from_millis(300));
        assert!(!s.is_alive());
    }
}
```

- [x] **Step 2: Correr el test para verificar que falla**

Run: `cargo test -p voice-type supervisor`
Expected: FALLA.

- [x] **Step 3: Implementación mínima**

```rust
use crate::ipc::{decode, encode, FromSidecar, ToSidecar};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::thread;

pub struct Sidecar {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<FromSidecar>,
}

impl Sidecar {
    pub fn spawn(exe: &Path) -> std::io::Result<Sidecar> {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null())
            .spawn()?;
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = channel();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                if let Ok(msg) = decode::<FromSidecar>(&line) {
                    if tx.send(msg).is_err() { break; }
                }
            }
        });
        Ok(Sidecar { child, stdin, rx })
    }

    pub fn send(&mut self, msg: &ToSidecar) {
        let _ = self.stdin.write_all(encode(msg).as_bytes());
        let _ = self.stdin.flush();
    }

    pub fn try_recv(&self) -> Option<FromSidecar> { self.rx.try_recv().ok() }

    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}
```

`Supervisor`: guarda `LoadModel` params; un tick cada 1s revisa `is_alive()` y última vez que se vio `Heartbeat` (actualizado al drenar `try_recv`); si falla, `Sidecar::spawn` + reenvía `LoadModel` con backoff.

- [x] **Step 4: Correr los tests**

Run: `cargo test -p voice-type supervisor`
Expected: PASA.

- [x] **Step 5: Commit**

```bash
git add src-tauri/src/supervisor.rs src-tauri/src/lib.rs src-tauri/tests/
git commit -m "feat: sidecar supervisor with spawn, IPC channels and liveness"
```

---

## Task 10: Módulo `text_injector` (portapapeles + Ctrl+V)

**Files:**
- Create: `src-tauri/src/text_injector.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml` (`arboard`, `enigo`)
- Test: unit para la lógica de save/restore con un clipboard mock (trait); pegado real = verificación manual.

**Interfaces:**
- Produces:
  - `trait Clipboard { fn get(&mut self) -> Option<String>; fn set(&mut self, s: &str); }`
  - `fn inject<C: Clipboard>(cb: &mut C, paste: impl FnOnce(), text: &str)` — guarda actual, set `text`, `paste()`, restaura.
  - Impl real `SystemClipboard` (arboard) y `fn paste_ctrl_v()` (enigo).

- [x] **Step 1: Escribir el test que falla**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct MockClip { value: Option<String>, log: RefCell<Vec<String>> }
    impl Clipboard for MockClip {
        fn get(&mut self) -> Option<String> { self.value.clone() }
        fn set(&mut self, s: &str) { self.log.borrow_mut().push(s.into()); self.value = Some(s.into()); }
    }

    #[test]
    fn inject_sets_text_then_restores_original() {
        let mut clip = MockClip { value: Some("ORIGINAL".into()), log: RefCell::new(vec![]) };
        let mut pasted = false;
        inject(&mut clip, || pasted = true, "hola mundo");
        assert!(pasted);
        let log = clip.log.borrow();
        assert_eq!(log[0], "hola mundo");   // primero pone el texto
        assert_eq!(log[1], "ORIGINAL");     // luego restaura
        assert_eq!(clip.value.as_deref(), Some("ORIGINAL"));
    }
}
```

- [x] **Step 2: Correr el test para verificar que falla**

Run: `cargo test -p voice-type text_injector`
Expected: FALLA.

- [x] **Step 3: Implementación mínima**

```rust
pub trait Clipboard {
    fn get(&mut self) -> Option<String>;
    fn set(&mut self, s: &str);
}

pub fn inject<C: Clipboard>(cb: &mut C, paste: impl FnOnce(), text: &str) {
    let original = cb.get();
    cb.set(text);
    paste();
    // pequeña espera real en producción antes de restaurar (ver SystemClipboard).
    if let Some(orig) = original { cb.set(&orig); }
}

pub struct SystemClipboard(arboard::Clipboard);
impl SystemClipboard {
    pub fn new() -> Self { Self(arboard::Clipboard::new().expect("clipboard")) }
}
impl Clipboard for SystemClipboard {
    fn get(&mut self) -> Option<String> { self.0.get_text().ok() }
    fn set(&mut self, s: &str) { let _ = self.0.set_text(s.to_string()); }
}

pub fn paste_ctrl_v() {
    use enigo::{Enigo, Key, Keyboard, Settings, Direction::{Press, Release}};
    let mut e = Enigo::new(&Settings::default()).unwrap();
    let _ = e.key(Key::Control, Press);
    let _ = e.key(Key::Unicode('v'), enigo::Direction::Click);
    let _ = e.key(Key::Control, Release);
}
```

Nota producción: entre `cb.set(text)` y restaurar, dormir ~120 ms para que la app destino procese el `Ctrl+V` antes de devolver el portapapeles original.

- [x] **Step 4: Correr tests unit**

Run: `cargo test -p voice-type text_injector`
Expected: PASA.

- [ ] **Step 5: Verificación manual**

Ejemplo `examples/inject_probe.rs`: foco en el Bloc de notas, correr, debe aparecer "hola mundo" y el portapapeles quedar como estaba.

> **Pendiente de verificación manual en Windows 11.** El ejemplo todavía no
> existe: `arboard`/`enigo` sólo compilan con la feature `app`, que no se puede
> construir en el entorno Linux donde se implementó el módulo. La lógica de
> guardar/restaurar sí está testeada, con portapapeles de mentira.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/text_injector.rs src-tauri/src/lib.rs src-tauri/Cargo.toml src-tauri/examples/inject_probe.rs
git commit -m "feat: text_injector with clipboard save/restore and Ctrl+V paste"
```

---

## Task 11: Pipeline de dictado (wiring en el proceso principal)

**Files:**
- Create: `src-tauri/src/pipeline.rs`
- Modify: `src-tauri/src/lib.rs`, `src-tauri/src/main.rs`
- Test: integración con fake sidecar (Task 9) + audio sintético (Task 8).

**Interfaces:**
- Consumes: `audio`, `vad`, `supervisor`, `post_processor`, `text_injector`, `vocab_store`.
- Produces:
  - `struct Pipeline` con `fn start(&mut self)` (arranca captura → segmenta → envía a sidecar) y `fn stop(&mut self)` (flush último segmento). Estado emitido a la UI: `Idle | Listening | Transcribing`.
  - `fn on_result(&mut self, id: u64, text: String)` — aplica `post_processor::apply` con reemplazos del `Vocab`, `join_segments`, e inyecta.

- [x] **Step 1: Escribir el test que falla**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocab_store::Vocab;

    #[test]
    fn on_result_applies_replacements_and_collects_text() {
        let mut v = Vocab::default();
        v.add("yeison", "JSON");
        let mut injected = String::new();
        let mut p = Pipeline::for_test(v, |t| injected.push_str(t));
        p.on_result(1, "mandame un yeison".into());
        assert_eq!(injected, "mandame un JSON");
    }
}
```

- [x] **Step 2: Correr el test para verificar que falla**

Run: `cargo test -p voice-type pipeline`
Expected: FALLA.

- [x] **Step 3: Implementación mínima**

`Pipeline::for_test` inyecta un closure en vez del `SystemClipboard` real. `on_result` hace:

```rust
pub fn on_result(&mut self, _id: u64, raw: String) {
    let text = crate::post_processor::apply(&raw, &self.vocab.replacements);
    (self.inject_fn)(&text);
}
```

El wiring real (`start`/`stop`) conecta: `AudioCapture` → canal → `Segmenter::push` → `Sidecar::send(Transcribe)`; un hilo drena `Sidecar::try_recv` → `on_result`. `stop` llama `Segmenter::flush` y procesa el último.

> **Parcial.** `on_result`, `finish`, `transcript`, `last_raw_segment` y
> `add_vocab_word` están implementados y testeados. El cableado con
> `AudioCapture` y `Sidecar` queda para Windows, porque depende de la captura
> pendiente de la Task 7.

- [x] **Step 4: Correr los tests**

Run: `cargo test -p voice-type pipeline`
Expected: PASA.

- [ ] **Step 5: Verificación manual end-to-end**

Con el sidecar real y modelo cargado: hablar, ver texto aparecer en el Bloc de notas por segmentos.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/pipeline.rs src-tauri/src/lib.rs src-tauri/src/main.rs
git commit -m "feat: dictation pipeline wiring audio->vad->stt->postproc->inject"
```

---

## Task 12: Hotkeys + comandos Tauri

**Files:**
- Create: `src-tauri/src/hotkeys.rs`, `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`, `src-tauri/Cargo.toml` (`tauri-plugin-global-shortcut`)
- Test: unit del parser de atajo; registro real = verificación manual.

**Interfaces:**
- Produces:
  - Comandos Tauri: `#[tauri::command] fn toggle_dictation(state) -> String` (devuelve nuevo estado), `fn add_vocab_word(heard, correct, state)`, `fn get_last_transcription(state) -> String`.
  - Registro del atajo `config.toggle_shortcut` que llama `toggle_dictation`.

- [x] **Step 1: Escribir el test que falla (mapeo estado toggle)**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toggle_flips_between_idle_and_listening() {
        let mut s = DictationState::Idle;
        s = s.toggled();
        assert_eq!(s, DictationState::Listening);
        s = s.toggled();
        assert_eq!(s, DictationState::Idle);
    }
}
```

- [x] **Step 2: Correr el test para verificar que falla**

Run: `cargo test -p voice-type hotkeys`
Expected: FALLA.

- [ ] **Step 3: Implementación mínima**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DictationState { Idle, Listening, Transcribing }

impl DictationState {
    pub fn toggled(self) -> Self {
        match self { DictationState::Idle => DictationState::Listening, _ => DictationState::Idle }
    }
}
```

`commands.rs`: comandos que mutan el `Pipeline` compartido (dentro de `tauri::State<Mutex<AppState>>`), emiten evento `state-changed` al frontend, y llaman `pipeline.start()/stop()`. Registrar `global-shortcut` en `main.rs` con `config.toggle_shortcut`.

> **Parcial.** `DictationState` con `toggled()` e `is_dictating()` está hecho, y
> hay un test que fija que sus nombres serializados coinciden con el tipo
> `Status` del frontend. `commands.rs` y el registro del atajo necesitan Tauri
> y quedan para Windows.

- [x] **Step 4: Correr los tests**

Run: `cargo test -p voice-type hotkeys`
Expected: PASA.

- [ ] **Step 5: Verificación manual**

Correr la app, apretar el atajo, ver el estado cambiar en la píldora.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/hotkeys.rs src-tauri/src/commands.rs src-tauri/src/main.rs src-tauri/Cargo.toml
git commit -m "feat: global toggle hotkey and Tauri commands"
```

---

## Task 13: Frontend — `PillWidget` (estética Apple)

**Files:**
- Create: `src/lib/PillWidget.svelte`, `src/lib/state.ts`
- Modify: `src/App.svelte`, `src/app.css`
- Test: manual visual + un test de store con Vitest.

**Interfaces:**
- Consumes: evento Tauri `state-changed` (`"Idle" | "Listening" | "Transcribing"`), comando `toggle_dictation`.
- Produces: ventana sin bordes, translúcida, redondeada, arrastrable; onda animada en `Listening`; botón "+" que abre el popup (Task 14).

- [x] **Step 1: Configurar ventana de la píldora**

En `tauri.conf.json`: ventana `decorations: false`, `transparent: true`, `alwaysOnTop: true`, `skipTaskbar: true`, tamaño chico (~ 220×56), `resizable: false`.

- [x] **Step 2: Store de estado + test Vitest**

```ts
// src/lib/state.ts
import { writable } from "svelte/store";
export type Status = "Idle" | "Listening" | "Transcribing";
export const status = writable<Status>("Idle");
```

```ts
// src/lib/state.test.ts
import { get } from "svelte/store";
import { status } from "./state";
import { test, expect } from "vitest";
test("default status is Idle", () => { expect(get(status)).toBe("Idle"); });
```

Run: `npx vitest run` → PASA.

- [x] **Step 3: Componente `PillWidget.svelte` (estética Apple)**

CSS: `backdrop-filter: blur(20px)`, `border-radius: 28px`, `background: rgba(255,255,255,0.65)` (y variante dark), sombra `0 8px 30px rgba(0,0,0,.18)`, `font-family: -apple-system, system-ui`. Estado `Listening`: barras de onda animadas con `@keyframes`. Escuchar el evento Tauri y actualizar `status`. Click en el cuerpo → `invoke("toggle_dictation")`. Botón "+" a la derecha.

- [ ] **Step 4: Verificación visual manual**

`npm run tauri dev`: la píldora se ve translúcida/redondeada; al togglear cambia el estado y anima.

> **Primera pasada hecha en navegador.** Con `npx vite preview` + capturas en
> Chromium se revisó el diseño en claro y oscuro: la píldora se ve translúcida
> y redondeada, las ondas animan en azul y el popup queda coherente. Falta la
> pasada en Windows: que el toggle real cambie el estado, y que la
> translucidez y el siempre-encima se comporten con una ventana de verdad.

- [x] **Step 5: Commit**

```bash
git add src/ tauri.conf.json src-tauri/tauri.conf.json
git commit -m "feat: Apple-style floating PillWidget with status states"
```

---

## Task 14: Frontend — `AddWordPopup` + wiring de vocabulario

**Files:**
- Create: `src/lib/AddWordPopup.svelte`
- Modify: `src/lib/PillWidget.svelte`, `src-tauri/src/commands.rs`
- Test: Vitest del componente (selección de palabra) + manual.

**Interfaces:**
- Consumes: comando `get_last_transcription` → string; comando `add_vocab_word(heard, correct)`.
- Produces: popup que muestra la última frase, permite tocar un token (marcarlo como `heard`) y escribir la forma `correct`, y guarda.

- [x] **Step 1: Test Vitest de tokenización/selección**

```ts
// src/lib/tokenize.test.ts
import { tokenize } from "./tokenize";
import { test, expect } from "vitest";
test("splits sentence into word tokens", () => {
  expect(tokenize("mandame un yeison")).toEqual(["mandame", "un", "yeison"]);
});
```

Crear `src/lib/tokenize.ts` con `export const tokenize = (s: string) => s.trim().split(/\s+/);`. Run: `npx vitest run` → PASA.

- [x] **Step 2: Componente `AddWordPopup.svelte`**

Al abrir: `invoke("get_last_transcription")`, render tokens como chips; al click en un chip se setea `heard`; input para `correct`; botón Guardar → `invoke("add_vocab_word", { heard, correct })` y cerrar. Estética: tarjeta redondeada translúcida, coherente con la píldora.

> **Steps 3 y 4 pendientes.** `get_last_transcription` y `add_vocab_word` son
> comandos de Tauri: la lógica que los alimenta ya está en `Pipeline`
> (`last_raw_segment()` y `add_vocab_word()`), falta exponerlos desde
> `commands.rs` en la máquina Windows.

- [ ] **Step 3: Backend commands**

Implementar `get_last_transcription` (última cadena guardada en `AppState`) y `add_vocab_word` (llama `Vocab::add`, `save`, y reenvía `LoadModel`/actualiza `initial_prompt` al sidecar en caliente).

- [ ] **Step 4: Verificación manual**

Dictar algo mal, "+", corregir palabra, guardar; volver a dictar y ver que ahora sale corregido (por reemplazo) y mejor reconocido (por prompt).

- [x] **Step 5: Commit**

```bash
git add src/ src-tauri/src/commands.rs
git commit -m "feat: AddWordPopup with vocab correction and hot prompt reload"
```

---

## Task 15: Servicio estable — tray, autostart, single-instance, logging, descarga de modelo

**Files:**
- Modify: `src-tauri/src/main.rs`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`
- Create: `src-tauri/src/model_download.rs`
- Test: unit del builder de URL/ruta del modelo; resto verificación manual.

**Interfaces:**
- Produces:
  - Tray con menú: Mostrar/ocultar píldora, Ajustes, Salir.
  - Autostart al login (`tauri-plugin-autostart`), instancia única (`tauri-plugin-single-instance`).
  - `fn model_path(app_data: &Path, name: &str) -> PathBuf` y `fn ensure_model(...)` (descarga si falta, con progreso).
  - Logging `tracing` a archivo con rotación diaria.

- [x] **Step 1: Test del path del modelo**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn model_path_joins_models_dir() {
        let p = model_path(Path::new("C:/data"), "ggml-large-v3-q5_0.bin");
        assert!(p.ends_with("models/ggml-large-v3-q5_0.bin") || p.ends_with("models\\ggml-large-v3-q5_0.bin"));
    }
}
```

- [x] **Step 2: Correr el test para verificar que falla**

Run: `cargo test -p voice-type model_download`
Expected: FALLA.

> **Parcial.** Lo que no toca la red está hecho y testeado: `model_path`,
> `partial_path`, `model_url` e `is_valid_model_name`. La descarga en sí
> (`ensure_model` con reqwest), la bandeja, el autostart, la instancia única y
> el logging necesitan Tauri y quedan para Windows.

- [ ] **Step 3: Implementación**

`model_path` une `app_data/models/<name>`. `ensure_model`: si no existe, descarga desde el repo oficial de modelos ggml de whisper.cpp (Hugging Face) con `reqwest` streaming + emite progreso al frontend. Registrar en `main.rs`: `tauri-plugin-autostart` (login), `tauri-plugin-single-instance`, tray con menú, y `tracing_subscriber` con `tracing-appender` (rotación diaria en `app_data/logs`).

- [x] **Step 4: Correr tests unit**

Run: `cargo test -p voice-type model_download`
Expected: PASA.

- [x] **Step 5: Verificación manual de servicio**

- Primer arranque sin modelo: descarga con progreso, luego dicta.
- Activar autostart; reiniciar sesión de Windows; la app arranca en la bandeja.
- Matar el proceso del sidecar desde el Administrador de tareas: el supervisor lo reinicia y el dictado sigue funcionando.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/
git commit -m "feat: tray, autostart, single-instance, logging and model download"
```

---

## Self-Review (cobertura del spec)

- §1 Resumen / dictado toggle + inyección → Tasks 11, 12.
- §2 Objetivos: español fijo + GPU → Task 6; vocab UI → Tasks 2, 14; estable/recuperación → Tasks 9, 15; UI Apple → Tasks 13, 14; offline → Global Constraints.
- §5 Stack completo → cubierto en Tasks 1–15.
- §6 Arquitectura supervisor+sidecar+IPC → Tasks 4, 6, 9.
- §7 Módulos: audio(7), vad(8), stt(6), vocab_store(2), post_processor(3), text_injector(10), hotkeys(12), supervisor(9), ipc(4), config(5), UI(13,14), commands(12,14).
- §8 Flujos dictado y "+" → Tasks 11, 14.
- §9 Vocabulario (bias + reemplazos) → Tasks 2, 3, 6 (initial_prompt), 14.
- §11 Config/persistencia → Task 5, 15.
- §12 Resiliencia (mic, CUDA fallback, restart, clipboard, descarga, single-instance) → Tasks 7, 9, 10, 15.
- §13 Autostart (no Windows Service) → Task 15.
- §14 Testing → tests en cada task.

**Gaps detectados y resueltos inline:**
- "hotwords" del spec se implementa vía `initial_prompt` de whisper.cpp (no existe param hotwords formal) — reflejado en Tasks 2 y 6.
- CUDA fallback a CPU: agregar en Task 6 producción un intento CPU si `WhisperContext` con CUDA falla (documentado; el mismo `transcribe_pcm` sirve, cambia solo el build del modelo/contexto). El worker emite `Error` y el supervisor puede relanzar el sidecar con variable de entorno `VT_FORCE_CPU=1`.

**Type consistency:** `ToSidecar`/`FromSidecar` usados igual en Tasks 4/6/9; `Vocab`/`Replacement` en 2/3/11/14; `DictationState` en 12/13; `Status` (frontend) espeja los nombres `Idle/Listening/Transcribing`.
