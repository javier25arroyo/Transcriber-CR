//! Sidecar de mentira para los tests del supervisor.
//!
//! Habla el mismo protocolo que el sidecar real pero sin modelo, sin GPU y sin
//! esperas: responde `Ready` a `LoadModel`, `Result { text: "ok" }` a
//! `Transcribe` y termina con `Shutdown`. Así los tests del supervisor prueban
//! spawn, IPC y detección de muerte en una máquina cualquiera.

use std::io::{BufRead, Write};

use voice_type::ipc::{decode, encode, FromSidecar, ToSidecar};

fn main() {
    let stdin = std::io::stdin();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let Ok(msg) = decode::<ToSidecar>(&line) else {
            continue;
        };
        match msg {
            ToSidecar::LoadModel { .. } => emit(&FromSidecar::Ready),
            ToSidecar::Transcribe { id, .. } => emit(&FromSidecar::Result {
                id,
                text: "ok".into(),
            }),
            ToSidecar::Shutdown => break,
        }
    }
}

fn emit(msg: &FromSidecar) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let _ = out.write_all(encode(msg).as_bytes());
    let _ = out.flush();
}
