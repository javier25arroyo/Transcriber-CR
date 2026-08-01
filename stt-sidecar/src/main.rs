//! Sidecar de STT: proceso aislado que carga el modelo de Whisper una vez y
//! transcribe los segmentos que le manda el proceso principal.
//!
//! Este archivo es sólo la cáscara: leer stdin, escribir stdout, latir. Qué
//! contestar a cada mensaje vive en `lib.rs`, que se testea sin procesos, y el
//! motor está detrás del trait `Engine`.

use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[cfg(not(feature = "whisper"))]
use stt_sidecar::UnavailableEngine;
use stt_sidecar::{handle, Engine, Flow};
use voice_type::ipc::{decode, encode, FromSidecar, ToSidecar};

/// Cada cuánto se manda señal de vida. El supervisor espera varios de estos
/// antes de dar por colgado al proceso (ver `voice_type::supervisor`).
const HEARTBEAT: Duration = Duration::from_secs(2);

fn main() {
    let stop = Arc::new(AtomicBool::new(false));
    spawn_heartbeat(Arc::clone(&stop));

    let mut engine = build_engine();

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        // Una línea ilegible no tira el proceso: se ignora y se sigue leyendo.
        // Cortar acá dejaría al supervisor reiniciando en loop por un mensaje
        // suelto mal formado.
        let Ok(msg) = decode::<ToSidecar>(&line) else {
            continue;
        };

        let (respuesta, flow) = handle(msg, engine.as_mut());
        if let Some(msg) = respuesta {
            emit(&msg);
        }
        if flow == Flow::Stop {
            break;
        }
    }

    stop.store(true, Ordering::Relaxed);
}

/// El motor real entra con la feature `whisper` (Task 6 del plan). Sin ella el
/// sidecar arranca igual y contesta un error claro, que es lo que el supervisor
/// sabe manejar.
#[cfg(feature = "whisper")]
fn build_engine() -> Box<dyn Engine> {
    Box::new(stt_sidecar::WhisperEngine::default())
}

/// Sin la feature `whisper`, el sidecar arranca igual pero contesta un error
/// claro a `LoadModel` y `Transcribe`. Así el crate compila y se testea en el
/// CI sin whisper.cpp ni sus dependencias de build (LLVM, CMake).
#[cfg(not(feature = "whisper"))]
fn build_engine() -> Box<dyn Engine> {
    Box::new(UnavailableEngine)
}

fn spawn_heartbeat(stop: Arc<AtomicBool>) {
    thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            thread::sleep(HEARTBEAT);
            if stop.load(Ordering::Relaxed) {
                break;
            }
            emit(&FromSidecar::Heartbeat);
        }
    });
}

/// Escribe un mensaje completo tomando el lock de stdout una sola vez: el
/// heartbeat corre en otro hilo y dos escrituras entremezcladas romperían el
/// framing de una línea por mensaje.
fn emit(msg: &FromSidecar) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(encode(msg).as_bytes());
    let _ = out.flush();
}
