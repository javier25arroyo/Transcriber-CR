# Voice Type — Documentos de ciclo de vida (SDLC) — Diseño

- **Fecha:** 2026-07-26
- **Estado:** Aprobado (brainstorming), pendiente plan de implementación
- **Autor:** Javier
- **Depende de:** [spec de producto](2026-07-25-voice-type-dictation-design.md) y [plan de implementación](../plans/2026-07-25-voice-type-dictation.md)

---

## 1. Resumen

Voice Type ya tiene spec de producto y plan de implementación de 15 tasks, pero
el repositorio no tiene ninguna infraestructura de proyecto: no es un repo git,
no hay licencia, ni README, ni CI, ni registro de decisiones.

Este diseño define **los documentos y la infraestructura de proceso** que
convierten la carpeta en un proyecto de software completo: desde que alguien
llega al repo hasta que una versión queda publicada. El producto en sí no
cambia; cambia el andamiaje alrededor.

El repositorio será **público desde el primer push**, con vistas a que sea
open-source utilizable por terceros.

## 2. Decisiones tomadas

| Decisión | Valor |
|---|---|
| Visibilidad del repo | Público desde el primer push |
| Nombre del repo | `Transcriber-CR` (ya creado en GitHub; el nombre del producto sigue siendo "Voice Type", el nombre del crate Rust sigue siendo `voice-type` — son tres identificadores independientes) |
| Licencia | MIT |
| Flujo de trabajo | Una rama por task + Pull Request a `main` |
| CI | GitHub Actions liviano: sin GPU, sin CUDA |
| Backlog de v1 | El plan markdown existente; GitHub Issues solo para bugs y features nuevas |
| Versionado | SemVer, arranca en `0.1.0`; `1.0.0` = las 15 tasks del plan completas y verificadas |
| Idioma | README bilingüe (inglés primero, español debajo); resto de documentos en inglés |
| Fixture de audio para tests | Generado con TTS del sistema, no voz real |
| Copyright y contacto | Javier Pérez Arroyo — javier25arojas@gmail.com |

> **Nota de privacidad:** el correo de contacto queda público e indexable de
> forma permanente en el historial de git. Decisión tomada a conciencia.

## 3. Inventario de artefactos

Ninguno de estos toca `docs/superpowers/`, que queda intacto.

```
Voice Type/
├─ README.md                       # bilingüe: qué es, requisitos, quick start
├─ LICENSE                         # MIT, © 2026 Javier
├─ CHANGELOG.md                    # Keep a Changelog + SemVer
├─ CONTRIBUTING.md                 # setup, convenciones, cómo correr tests
├─ CODE_OF_CONDUCT.md              # Contributor Covenant 2.1
├─ SECURITY.md                     # modelo de amenaza y reporte de vulnerabilidades
├─ .gitignore                      # Rust, Node, Tauri, modelos, fixtures generados
├─ .gitattributes                  # normaliza finales de línea, marca binarios
├─ .github/
│  ├─ workflows/ci.yml             # fmt + clippy + tests sin GPU + checks de frontend
│  ├─ ISSUE_TEMPLATE/
│  │  ├─ bug_report.md
│  │  ├─ feature_request.md
│  │  └─ config.yml                # desactiva issues en blanco, enlaza a Discussions
│  └─ PULL_REQUEST_TEMPLATE.md
└─ docs/
   ├─ adr/
   │  ├─ README.md                 # índice y explicación del formato
   │  ├─ 0001-isolated-stt-sidecar.md
   │  ├─ 0002-toggle-over-push-to-talk.md
   │  ├─ 0003-clipboard-paste-injection.md
   │  ├─ 0004-fixed-spanish-with-custom-vocabulary.md
   │  └─ 0005-tauri-v2-and-svelte-stack.md
   └─ superpowers/                 # ya existe, sin cambios
```

## 4. Contenido de cada documento

### 4.1 `README.md` (bilingüe)

Inglés primero, español debajo del mismo contenido, separados por un `---` y
enlaces de salto al inicio. Secciones:

- **Qué es:** dictado por voz (voz → texto) para Windows 11, español, offline,
  con aceleración por GPU. Atajo global, el texto aparece en la app enfocada.
- **Estado:** v1 en desarrollo — enlace al plan y al CHANGELOG.
- **Requisitos:** Windows 11, Rust estable, Node LTS, CUDA Toolkit 12.x, GPU
  NVIDIA con 8 GB de VRAM (fallback a CPU disponible, más lento).
- **Quick start:** `npm install`, `npm run tauri dev`. El modelo whisper se
  descarga solo en el primer arranque.
- **Cómo funciona:** cinco líneas, con el diagrama de supervisor + sidecar
  copiado del spec de producto.
- **Vocabulario personalizado:** qué resuelve el botón "+".
- **Licencias de terceros:** whisper.cpp (MIT) y los modelos ggml de OpenAI
  (MIT), aclarando que el modelo no se distribuye en el repo sino que se
  descarga en tiempo de ejecución.
- **Enlaces:** spec de producto, plan de implementación, ADRs, CONTRIBUTING.

El mantenimiento de las dos versiones es responsabilidad de quien edita: si
cambia una, cambia la otra en el mismo commit. La plantilla de PR lo recuerda.

### 4.2 `CONTRIBUTING.md` (inglés)

- Setup del entorno paso a paso, con los comandos de verificación
  (`rustc --version`, `node --version`, `nvcc --version`).
- Layout del workspace: `src-tauri` (lib + binario principal), `stt-sidecar`
  (binario aislado), `src` (frontend Svelte). Aclara la **dirección de la
  dependencia**: el sidecar depende de `voice-type`, nunca al revés.
- Convenciones: Rust edición 2021, `rustfmt` por defecto, `clippy` sin
  warnings, commits `feat:` / `fix:` / `test:` / `chore:`.
- TDD obligatorio en lógica pura y determinista (`vocab_store`,
  `post_processor`, `ipc`, `config`, `vad`).
- Los tres niveles de test y cómo correr cada uno:
  - `cargo test -p voice-type` — lógica pura, sin hardware. Corre en CI.
  - `cargo test -p stt-sidecar -- --ignored` — necesita modelo local y CUDA. Solo local.
  - `npx vitest run` — frontend. Corre en CI.
- Qué se verifica a mano y no puede automatizarse: captura de micrófono,
  inyección de texto en otra aplicación, uso real de GPU, autostart al login,
  reinicio del sidecar tras matarlo desde el Administrador de tareas.
- Flujo de rama + PR (ver §6).

### 4.3 `SECURITY.md` (inglés)

Modelo de amenaza breve y honesto, porque la app tiene permisos incómodos:

- **Qué hace la app:** captura audio del micrófono, escribe en el portapapeles
  del sistema y simula pulsaciones de teclado (`Ctrl+V`).
- **Qué no hace:** no envía nada por la red salvo la descarga inicial del modelo
  desde Hugging Face. Sin telemetría, sin cuentas, sin sincronización.
- **Riesgos reconocidos:**
  - El portapapeles se sobrescribe momentáneamente durante cada inyección; se
    restaura después, pero un gestor de portapapeles de terceros puede capturar
    el valor intermedio.
  - El texto dictado se inyecta en la ventana que tenga el foco. Si el foco
    cambia entre que se habla y que se pega, el texto va a la ventana
    equivocada.
  - Los logs **no deben contener transcripciones ni audio**. Esto es una regla
    de implementación, no una sugerencia.
- **Reporte de vulnerabilidades:** por GitHub Security Advisories (canal
  privado), no por issue público. Contacto alternativo: javier25arojas@gmail.com.
- **Alcance de soporte:** solo la última versión publicada.

### 4.4 `CHANGELOG.md` (inglés)

Formato Keep a Changelog con SemVer. Arranca con una única sección
`[Unreleased]` con las subsecciones vacías `Added` / `Changed` / `Fixed`.
Se actualiza en el mismo PR que introduce el cambio, no al final.

### 4.5 `CODE_OF_CONDUCT.md` (inglés)

Contributor Covenant 2.1 sin modificaciones, contacto javier25arojas@gmail.com.

### 4.6 `LICENSE`

MIT, `Copyright (c) 2026 Javier Pérez Arroyo`.

### 4.7 `.gitignore`

Cubre: `target/`, `node_modules/`, `dist/`, artefactos de build de Tauri,
`models/` (los `.bin` de whisper pesan cientos de MB y se descargan solos),
logs locales, y archivos de configuración de la app en desarrollo. El fixture de
audio sintético **sí** se versiona (ver §10).

### 4.8 `.gitattributes`

El desarrollo es en Windows pero el repo es público y el job de frontend del CI
corre en Linux. Sin normalización, los finales de línea generan diffs completos
sobre archivos que nadie tocó.

- `* text=auto eol=lf` — todo texto se guarda con LF en el repositorio.
- `*.ps1 text eol=crlf` — los scripts de PowerShell necesitan CRLF.
- `*.wav binary`, `*.bin binary`, `*.png binary` — sin conversión ni diffs.

## 5. Registro de decisiones (ADRs)

Formato corto y fijo: **Contexto / Decisión / Consecuencias / Alternativas
descartadas**. Cada ADR tiene estado `Accepted` y fecha `2026-07-26`, y
referencia la sección del spec de producto de donde sale la decisión, en vez de
duplicar su texto.

| ADR | Decisión | Origen |
|---|---|---|
| 0001 | El STT corre en un proceso sidecar aislado, supervisado y reiniciable | spec §6 |
| 0002 | Activación por toggle, no push-to-talk | spec §3, §8.1 |
| 0003 | Inyección de texto vía portapapeles + `Ctrl+V`, no simulación tecla a tecla | spec §5, §12 |
| 0004 | Idioma fijo español; el inglés entra por vocabulario personalizado | spec §3, §9 |
| 0005 | Stack Tauri v2 + Svelte + `whisper-rs` | spec §5 |

El ADR 0001 incluye una consecuencia que el resto del proyecto debe respetar:
**la dependencia va del sidecar hacia `voice-type`, nunca al revés**. Invertirla
haría que el crate principal arrastre `whisper-rs` con CUDA y rompería el CI,
que no tiene toolkit de CUDA disponible.

`docs/adr/README.md` explica el formato, cómo numerar uno nuevo y cuándo
escribirlo: solo para decisiones caras de revertir.

## 6. Flujo de trabajo

**Rama por task, PR a `main`.**

- Nombre de rama: `task/NN-descripcion-corta` para las tasks del plan (por
  ejemplo `task/02-vocab-store`), `fix/...` o `feat/...` para lo demás.
- El PR usa la plantilla, que incluye: qué cambia, cómo se verificó, checkbox de
  CHANGELOG actualizado, y checkbox de "si toqué el README, actualicé ambos
  idiomas".
- `main` protegida: requiere que el CI pase antes de mergear. Sin revisor
  obligatorio, porque el proyecto es de un solo desarrollador; cuando aparezca
  alguien más se agrega esa regla.
- Merge por squash, para que cada task quede como un commit en `main`.

**Plantillas de issue:** `bug_report.md` (versión, pasos, comportamiento
esperado, logs relevantes con la advertencia de no pegar transcripciones
personales) y `feature_request.md` (problema que resuelve, alternativas
consideradas). `config.yml` desactiva los issues en blanco.

## 7. Integración continua

Un único workflow, `.github/workflows/ci.yml`, en `push` a `main` y en cada PR:

**Job `rust`** — `windows-latest`:
1. `cargo fmt --all -- --check`
2. `cargo clippy -p voice-type --all-targets -- -D warnings`
3. `cargo test -p voice-type`

Deliberadamente **excluye `stt-sidecar`**: compilar `whisper-rs` con la feature
`cuda` requiere el CUDA Toolkit, que no está en los runners de GitHub. Los tests
que necesitan modelo están marcados `#[ignore]` y solo corren en local.

**Job `frontend`** — `ubuntu-latest`:
1. `npm ci`
2. `npx vitest run`
3. `npx svelte-check`

El workflow se agrega antes de que exista el código. Cada paso que invoca
`cargo` o `npm` está condicionado a `hashFiles('Cargo.toml') != ''` /
`hashFiles('package.json') != ''`: si el manifiesto no existe todavía, el paso
se salta (el job queda en verde) en vez de fallar. La guarda se retira en la
Task 1 del plan, cuando el scaffold crea esos manifiestos — a partir de ahí,
la ausencia de `Cargo.toml` o `package.json` sí debe hacer fallar el CI.

## 8. Versionado y releases

- SemVer. La versión vive en `src-tauri/Cargo.toml` y `package.json`, y ambas se
  mantienen sincronizadas a mano.
- Arranca en `0.1.0` cuando el scaffold compila y abre una ventana.
- Cada hito del plan que deje la app usable sube la minor: `0.2.0` dictado
  end-to-end funcionando, `0.3.0` vocabulario personalizado, y así.
- `1.0.0` cuando las 15 tasks del plan estén completas y verificadas: dictado,
  vocabulario, y servicio estable con autostart y recuperación del sidecar.
- Un release se corta con un tag `vX.Y.Z`, moviendo la sección `[Unreleased]`
  del CHANGELOG a una sección con número y fecha. Sin binarios publicados en v1:
  el spec de producto excluye explícitamente el instalador distribuible.

## 9. Orden de ejecución

1. `git init`, `.gitignore`, commit inicial con los `docs/` que ya existen.
2. Documentos raíz, ADRs y `.github/` → commit.
3. Repo `Transcriber-CR` en GitHub (ya creado y con remoto configurado), push,
   activar protección de `main` requiriendo CI en verde.
4. Recién entonces, Task 1 del plan de implementación (scaffold Tauri).

El orden importa: la protección de rama se configura antes de la primera rama de
trabajo, no después.

## 10. Cambios al plan de implementación existente

Las decisiones de este diseño modifican cuatro puntos del plan del 2026-07-25.
Estos cambios se aplican al plan cuando se ejecute:

1. **Global Constraints** — donde dice "commits frecuentes, uno por tarea como
   mínimo", ahora además: cada task va en su propia rama y entra a `main` por PR
   con CI en verde.
2. **Task 1, Step 1** — el `git init` ya estará hecho por el paso 1 de §9. La
   task arranca directamente en el scaffold de Tauri, sobre una rama
   `task/01-scaffold`.
3. **Task 6, Step 2** — el fixture `hola.wav` no se graba con voz real: se
   genera con el TTS del sistema (`System.Speech` de Windows vía PowerShell) a
   16 kHz mono. El generador se versiona en
   `scripts/generate-test-fixtures.ps1`; el `.wav` resultante también se
   versiona, por ser sintético y no contener datos personales. Se documenta en
   CONTRIBUTING cómo regenerarlo.
4. **Task 1** — agrega un step final: retirar de `.github/workflows/ci.yml`
   los `if: hashFiles(...) != ''` en los pasos de `cargo` y `npm` (ver §7),
   ahora que `Cargo.toml` y `package.json` existen. El CI debe volver a fallar
   ante la ausencia de esos archivos, no saltear el paso en silencio.

Además, cada task del plan pasa a incluir la actualización del CHANGELOG dentro
de su PR.

## 11. No-objetivos (YAGNI)

- Sin `CODEOWNERS`, sin proceso de RFC, sin plantilla de governance.
- Sin runner self-hosted con GPU: el CI no compila CUDA ni corre tests de modelo.
- Sin publicación de binarios ni instalador firmado (fuera del spec de v1).
- Sin migrar el plan de 15 tasks a GitHub Issues; queda como markdown.
- Sin Dependabot ni escaneo automático de dependencias en v1.
- Sin documentación de API generada (`cargo doc` publicado): el proyecto es una
  aplicación, no una biblioteca.

## 12. Riesgos

- **README bilingüe se desincroniza.** Mitigado con un checkbox en la plantilla
  de PR; sigue dependiendo de la disciplina de quien edita.
- **Las guardas de CI se olvidan de retirar.** Si el scaffold llega y `Cargo.toml`
  o `package.json` existen pero el guard sigue ahí por descuido, un paso roto
  se saltearía en silencio en vez de fallar. Mitigado: la Task 1 del plan
  incluye explícitamente retirar la guarda como parte del checklist.
  para que no se confunda con una falla real.
- **CI da cobertura parcial:** todo lo que toca GPU, micrófono, portapapeles y
  ventanas queda fuera. La verificación manual listada en CONTRIBUTING es la
  única red de seguridad para esa parte, y es fácil de saltear.
- **El correo de contacto queda público** de forma permanente en el historial.
