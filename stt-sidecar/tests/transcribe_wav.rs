//! Test de integración del motor real: carga un modelo de whisper.cpp y
//! transcribe un wav de voz, verificando que el texto sale bien.
//!
//! Va `#[ignore]` porque necesita el modelo local (`models/ggml-base.bin`, ~142
//! MB, no versionado) y corre whisper.cpp de verdad, cosa que no está en el CI.
//! Para correrlo, con el modelo descargado:
//!
//! ```text
//! cargo test -p stt-sidecar --features whisper -- --ignored
//! ```
//!
//! El fixture `hola_mundo.wav` se generó con la síntesis de voz de Windows
//! (SAPI) a 16 kHz mono; ver `docs/progress.md`, Task 6.
#![cfg(feature = "whisper")]

use stt_sidecar::{Engine, WhisperEngine};

#[test]
#[ignore = "requiere models/ggml-base.bin; correr con --ignored"]
fn transcribes_reference_wav() {
    let fixture = format!(
        "{}/tests/fixtures/hola_mundo.wav",
        env!("CARGO_MANIFEST_DIR")
    );
    let model = format!("{}/../models/ggml-base.bin", env!("CARGO_MANIFEST_DIR"));

    let pcm = load_wav_16k_mono(&fixture);

    let mut engine = WhisperEngine::default();
    engine
        .load(&model, "es", "")
        .expect("el modelo de referencia tiene que cargar");

    let text = engine
        .transcribe(&pcm)
        .expect("la transcripción no debería fallar")
        .to_lowercase();

    assert!(
        text.contains("hola"),
        "esperaba oír «hola» en la transcripción, salió: «{text}»"
    );
}

/// Lee un wav de 16 kHz mono de 16 bits y lo devuelve como muestras `f32` en
/// [-1, 1], que es lo que espera whisper.cpp.
fn load_wav_16k_mono(path: &str) -> Vec<f32> {
    let mut reader = hound::WavReader::open(path)
        .unwrap_or_else(|e| panic!("no se pudo abrir el fixture {path}: {e}"));
    let spec = reader.spec();
    assert_eq!(spec.sample_rate, 16_000, "el fixture tiene que ser 16 kHz");
    assert_eq!(spec.channels, 1, "el fixture tiene que ser mono");

    reader
        .samples::<i16>()
        .map(|s| s.expect("muestra ilegible en el wav") as f32 / 32768.0)
        .collect()
}
