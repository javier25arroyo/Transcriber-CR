//! Verificación manual de la captura de micrófono (Task 7 del plan).
//!
//! Graba 3 segundos del dispositivo de entrada por defecto y muestra cuántas
//! muestras mono a 16 kHz llegaron y su RMS. Hablar durante la captura tiene
//! que subir el RMS bien por encima de 0; en silencio debería quedar cerca de
//! 0. No es un test automático — `cpal` necesita un micrófono real, así que
//! esto se corre y se mira a mano.
//!
//! Run: `cargo run -p voice-type --example mic_probe --features capture`

use std::sync::mpsc::channel;
use std::time::Duration;
use voice_type::audio::AudioCapture;

fn main() {
    let (tx, rx) = channel::<Vec<f32>>();

    let capture = AudioCapture::start(None, tx).expect("no se pudo arrancar la captura");
    println!("Capturando 3 segundos del micrófono por defecto. Hablá ahora...");

    let mut samples: Vec<f32> = Vec::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while std::time::Instant::now() < deadline {
        while let Ok(block) = rx.try_recv() {
            samples.extend(block);
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    // Drenar lo que haya quedado en el canal después del último sleep.
    while let Ok(block) = rx.try_recv() {
        samples.extend(block);
    }
    drop(capture);

    let rms = if samples.is_empty() {
        0.0
    } else {
        (samples.iter().map(|s| s * s).sum::<f32>() / samples.len() as f32).sqrt()
    };

    println!("Muestras capturadas (16 kHz mono): {}", samples.len());
    println!("RMS: {rms:.4}");
    if rms <= 0.0 {
        eprintln!("RMS en cero: revisar que el micrófono esté conectado y sin mute.");
    }
}
