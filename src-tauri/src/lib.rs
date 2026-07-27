//! Librería `voice_type`: la lógica del proceso principal, expuesta aparte del
//! binario para poder testearla con `cargo test` sin levantar la UI.
//!
//! Todo lo que vive acá es puro y determinista, o depende sólo de `std`. Las
//! partes que hablan con el hardware (micrófono, portapapeles, atajos globales)
//! aíslan el efecto detrás de un trait o de una función chica, y el resto —el
//! algoritmo— se testea sin hardware. Ver CONTRIBUTING.md § Running tests.

pub mod config;
pub mod ipc;
pub mod post_processor;
pub mod supervisor;
pub mod vad;
pub mod vocab_store;

/// Punto de entrada de la app Tauri. Vive en la librería para que `main.rs`
/// quede en una sola línea, como recomienda la plantilla de Tauri v2.
#[cfg(feature = "app")]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error al arrancar la aplicación Tauri");
}
