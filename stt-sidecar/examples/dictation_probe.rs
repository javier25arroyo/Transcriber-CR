//! Prueba manual de dictado de punta a punta, en la terminal.
//!
//! Captura el micrófono, corta en segmentos por silencio y transcribe cada uno
//! con el motor real de whisper.cpp, imprimiendo el texto a medida que sale.
//! Es la forma de confirmar que el `WhisperEngine` de la Task 6 entiende voz de
//! verdad, sin necesitar todavía el cableado Tauri (Tasks 11-12).
//!
//! Uso:
//!
//! ```text
//! cargo run -p stt-sidecar --example dictation_probe \
//!     --features "whisper capture" -- models/ggml-base.bin es
//! ```
//!
//! El primer argumento es la ruta al modelo `ggml-*.bin`; el segundo, opcional,
//! el idioma (por defecto `es`). Cortar con Ctrl+C.

use std::sync::mpsc;

use stt_sidecar::{Engine, WhisperEngine};
use voice_type::audio::AudioCapture;
use voice_type::vad::Segmenter;

/// El micrófono se captura y transcribe a 16 kHz mono (lo que entrega
/// `AudioCapture` y lo que espera whisper.cpp).
const SAMPLE_RATE: u32 = 16_000;

/// Cuánto silencio cierra un segmento. 700 ms es una pausa natural entre frases
/// sin cortar en medio de una.
const SILENCE_MS: u32 = 700;

fn main() {
    // whisper.cpp escupe sus propios logs de carga y de cada decodificación por
    // stderr. Redirigirlos a los hooks de whisper-rs, compilado sin las features
    // `log`/`tracing`, los apaga: en la terminal queda sólo el texto dictado.
    // (El sidecar de producción, en cambio, deja esos logs a propósito para ver
    // si agarró CUDA o cayó a CPU.)
    whisper_rs::install_logging_hooks();

    let mut args = std::env::args().skip(1);
    let model_path = args.next().unwrap_or_else(|| {
        eprintln!("uso: dictation_probe <ruta-al-modelo.bin> [idioma]");
        std::process::exit(2);
    });
    let language = args.next().unwrap_or_else(|| "es".to_string());

    eprintln!("Cargando modelo «{model_path}» (idioma: {language})...");
    let mut engine = WhisperEngine::default();
    if let Err(e) = engine.load(&model_path, &language, "") {
        eprintln!("no se pudo cargar el modelo: {e}");
        std::process::exit(1);
    }

    let (tx, rx) = mpsc::channel::<Vec<f32>>();
    let _capture = match AudioCapture::start(None, tx) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("no se pudo abrir el micrófono: {e}");
            std::process::exit(1);
        }
    };

    let mut segmenter = Segmenter::new(SILENCE_MS, SAMPLE_RATE);

    eprintln!("Escuchando. Hablá al micrófono; cada pausa cierra una frase. Ctrl+C para salir.\n");

    // El micrófono manda bloques a medida que el driver los entrega; el
    // segmentador los acumula y devuelve un segmento cerrado cuando detecta
    // silencio después de voz. `recv` termina en error sólo si la captura murió.
    for block in rx {
        if let Some(segment) = segmenter.push(&block) {
            transcribe_and_print(&mut engine, &segment);
        }
    }

    // La captura se cayó: cerrar lo que haya quedado a medias antes de salir.
    if let Some(segment) = segmenter.flush() {
        transcribe_and_print(&mut engine, &segment);
    }
}

fn transcribe_and_print(engine: &mut WhisperEngine, segment: &[f32]) {
    match engine.transcribe(segment) {
        Ok(text) if !text.is_empty() => println!("> {text}"),
        Ok(_) => {} // Segmento sin habla reconocible: no imprimir línea vacía.
        Err(e) => eprintln!("[error de transcripción] {e}"),
    }
}
