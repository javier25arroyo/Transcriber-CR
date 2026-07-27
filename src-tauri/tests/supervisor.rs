//! Supervisor contra un sidecar de mentira (`tests/bin/echo_sidecar.rs`).
//!
//! Es un test de integración y no unitario a propósito: necesita el binario
//! compilado, y `CARGO_BIN_EXE_*` sólo existe para los tests de integración.

use std::path::Path;
use std::time::{Duration, Instant};

use voice_type::ipc::{FromSidecar, ToSidecar};
use voice_type::supervisor::Sidecar;

const FAKE_SIDECAR: &str = env!("CARGO_BIN_EXE_echo-sidecar");

/// Espera hasta 5 s a que llegue un mensaje que cumpla el predicado.
fn wait_for(sidecar: &Sidecar, matches: impl Fn(&FromSidecar) -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        match sidecar.try_recv() {
            Some(msg) if matches(&msg) => return true,
            Some(_) => continue,
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    false
}

#[test]
fn spawn_send_receive_with_fake_sidecar() {
    let mut sidecar = Sidecar::spawn(Path::new(FAKE_SIDECAR)).unwrap();

    sidecar.send(&ToSidecar::LoadModel {
        model_path: "x".into(),
        language: "es".into(),
        initial_prompt: String::new(),
    });
    assert!(wait_for(&sidecar, |m| matches!(m, FromSidecar::Ready)));

    sidecar.send(&ToSidecar::Transcribe {
        id: 1,
        pcm16k: vec![0.0; 10],
    });
    assert!(wait_for(&sidecar, |m| matches!(
        m,
        FromSidecar::Result { id: 1, .. }
    )));
}

#[test]
fn detects_dead_process() {
    let mut sidecar = Sidecar::spawn(Path::new(FAKE_SIDECAR)).unwrap();
    assert!(sidecar.is_alive());

    sidecar.send(&ToSidecar::Shutdown);
    let deadline = Instant::now() + Duration::from_secs(5);
    while sidecar.is_alive() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!sidecar.is_alive(), "el proceso tenía que haber terminado");
}

#[test]
fn a_replacement_sidecar_serves_requests_again() {
    // Es lo que hace el supervisor después de un crash: levanta uno nuevo y
    // sigue como si nada.
    let mut dead = Sidecar::spawn(Path::new(FAKE_SIDECAR)).unwrap();
    dead.send(&ToSidecar::Shutdown);

    let mut fresh = Sidecar::spawn(Path::new(FAKE_SIDECAR)).unwrap();
    fresh.send(&ToSidecar::Transcribe {
        id: 42,
        pcm16k: vec![0.0; 4],
    });
    assert!(wait_for(&fresh, |m| matches!(
        m,
        FromSidecar::Result { id: 42, .. }
    )));
}

#[test]
fn spawning_a_missing_binary_is_an_error_not_a_panic() {
    let missing = Path::new("no/existe/sidecar-inventado");
    assert!(Sidecar::spawn(missing).is_err());
}
