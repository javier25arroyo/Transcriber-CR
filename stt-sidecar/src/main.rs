//! Sidecar de STT: proceso aislado que carga el modelo de Whisper una vez y
//! transcribe los segmentos que le manda el proceso principal.
//!
//! Acá está el esqueleto del loop de IPC y el heartbeat. El motor real
//! (`whisper-rs` con CUDA) se enchufa en la Task 6 del plan de implementación;
//! hasta entonces `Transcribe` responde `Error`, que es exactamente lo que el
//! supervisor sabe manejar.

use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use voice_type::ipc::{decode, encode, FromSidecar, ToSidecar};

/// Cada cuánto se manda señal de vida. El supervisor espera varios de estos
/// antes de dar por colgado al proceso (ver Task 9).
const HEARTBEAT: Duration = Duration::from_secs(2);

fn main() {
    let stop = Arc::new(AtomicBool::new(false));
    spawn_heartbeat(Arc::clone(&stop));

    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        // Una línea ilegible no tira el proceso: se ignora y se sigue leyendo.
        // Cortar acá dejaría al supervisor reiniciando en loop por un mensaje
        // suelto mal formado.
        let Ok(msg) = decode::<ToSidecar>(&line) else {
            continue;
        };

        match msg {
            ToSidecar::LoadModel { .. } => emit(&FromSidecar::Error {
                message: "motor STT todavía no implementado (Task 6 del plan)".into(),
            }),
            ToSidecar::Transcribe { .. } => emit(&FromSidecar::Error {
                message: "motor STT todavía no implementado (Task 6 del plan)".into(),
            }),
            ToSidecar::Shutdown => break,
        }
    }

    stop.store(true, Ordering::Relaxed);
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
