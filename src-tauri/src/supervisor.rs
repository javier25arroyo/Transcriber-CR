//! Supervisión del sidecar de STT: lanzarlo, hablarle, y darse cuenta cuando
//! se muere o se cuelga para reiniciarlo.
//!
//! Lo que rodea al proceso hijo (spawn, tuberías) se prueba en
//! `tests/supervisor.rs` contra un sidecar de mentira. La política de reinicio
//! vive acá y se prueba sin procesos ni relojes: recibe el tiempo transcurrido
//! como parámetro en vez de mirar el reloj por su cuenta.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::thread;
use std::time::Duration;

use crate::ipc::{decode, encode, FromSidecar, ToSidecar};

/// Techo de la espera entre reintentos. Si el sidecar no arranca por algo que
/// no se va a arreglar solo (falta el modelo, no hay VRAM), reintentar cada
/// 5 s alcanza para recuperarse apenas se resuelva, sin quemar CPU.
pub const MAX_BACKOFF: Duration = Duration::from_secs(5);

/// Primera espera de la serie: 0.5 s, 1 s, 2 s, 4 s, y de ahí [`MAX_BACKOFF`].
const FIRST_BACKOFF: Duration = Duration::from_millis(500);

/// Un sidecar corriendo: el proceso hijo, su stdin y los mensajes que llegan
/// por stdout, ya deserializados.
pub struct Sidecar {
    child: Child,
    stdin: ChildStdin,
    rx: Receiver<FromSidecar>,
}

impl Sidecar {
    /// Lanza el sidecar. `stderr` queda heredado a propósito: los logs de
    /// whisper.cpp (que dicen si agarró CUDA o cayó a CPU) son la única forma
    /// de diagnosticar el motor, y perderlos deja ciego al usuario.
    pub fn spawn(exe: &Path) -> std::io::Result<Sidecar> {
        let mut child = Command::new(exe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;

        let stdin = child
            .stdin
            .take()
            .expect("stdin fue configurado como tubería");
        let stdout = child
            .stdout
            .take()
            .expect("stdout fue configurado como tubería");

        // Leer en un hilo aparte: el proceso principal nunca puede quedarse
        // esperando a un sidecar que dejó de responder.
        let (tx, rx) = channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                // Una línea ilegible se ignora: puede ser ruido que el motor
                // nativo escribió en stdout.
                if let Ok(msg) = decode::<FromSidecar>(&line) {
                    // Si el receptor se fue, este sidecar ya no le importa a
                    // nadie y el hilo termina.
                    if tx.send(msg).is_err() {
                        break;
                    }
                }
            }
        });

        Ok(Sidecar { child, stdin, rx })
    }

    /// Manda un mensaje. Un error de escritura no se propaga: significa que el
    /// sidecar murió, y de eso ya se encarga el watchdog en el próximo tick.
    pub fn send(&mut self, msg: &ToSidecar) {
        let _ = self.stdin.write_all(encode(msg).as_bytes());
        let _ = self.stdin.flush();
    }

    pub fn try_recv(&self) -> Option<FromSidecar> {
        self.rx.try_recv().ok()
    }

    pub fn is_alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }
}

impl Drop for Sidecar {
    fn drop(&mut self) {
        // Sin esto, un sidecar reemplazado seguiría vivo ocupando VRAM.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Espera creciente entre reintentos.
#[derive(Debug, Default)]
pub struct Backoff {
    attempts: u32,
}

impl Backoff {
    /// Cuánto esperar antes del próximo intento, y avanza la serie.
    ///
    /// No se llama `next` para no confundirse con `Iterator::next`, que es lo
    /// que espera cualquiera que lea `backoff.next()`.
    pub fn next_delay(&mut self) -> Duration {
        let delay = FIRST_BACKOFF
            .checked_mul(2u32.saturating_pow(self.attempts))
            .unwrap_or(MAX_BACKOFF)
            .min(MAX_BACKOFF);
        self.attempts = self.attempts.saturating_add(1);
        delay
    }

    pub fn reset(&mut self) {
        self.attempts = 0;
    }
}

/// Decide cuándo hay que reiniciar el sidecar.
///
/// No mira el reloj: `tick` recibe cuánto tiempo pasó desde la llamada
/// anterior. Eso lo vuelve determinista y testeable sin dormir el test.
pub struct Watchdog {
    silence_timeout: Duration,
    silent_for: Duration,
    backoff: Backoff,
    /// El primer reinicio de una racha es inmediato; la espera creciente
    /// aparece recién a partir del segundo.
    restarted_once: bool,
}

impl Watchdog {
    pub fn new(silence_timeout: Duration) -> Self {
        Self {
            silence_timeout,
            silent_for: Duration::ZERO,
            backoff: Backoff::default(),
            restarted_once: false,
        }
    }

    /// Llamar con cada mensaje recibido del sidecar, incluido el heartbeat.
    /// Un sidecar que volvió a hablar arranca de cero: la próxima caída no
    /// arrastra la espera acumulada por las anteriores.
    pub fn on_message(&mut self) {
        self.silent_for = Duration::ZERO;
        self.backoff.reset();
        self.restarted_once = false;
    }

    /// Devuelve `Some(espera)` si hay que relanzar el sidecar (después de esa
    /// espera), o `None` si está sano.
    ///
    /// Se reinicia por dos motivos: el proceso murió, o sigue existiendo pero
    /// hace demasiado que no dice nada. El segundo caso es el importante: un
    /// cuelgue de CUDA deja el proceso vivo y mudo, y mirar sólo si está vivo
    /// no lo detectaría nunca.
    ///
    /// El primer reinicio de una racha no espera: hacer esperar al usuario
    /// después de un crash aislado sólo alarga el rato en que el dictado no
    /// funciona. La espera creciente es para el sidecar que no logra arrancar.
    pub fn tick(&mut self, elapsed: Duration, alive: bool) -> Option<Duration> {
        self.silent_for += elapsed;
        if alive && self.silent_for < self.silence_timeout {
            return None;
        }
        self.silent_for = Duration::ZERO;

        let delay = if self.restarted_once {
            self.backoff.next_delay()
        } else {
            Duration::ZERO
        };
        self.restarted_once = true;
        Some(delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TIMEOUT: Duration = Duration::from_secs(6);

    #[test]
    fn backoff_grows_and_then_stops_growing() {
        let mut b = Backoff::default();
        assert_eq!(b.next_delay(), Duration::from_millis(500));
        assert_eq!(b.next_delay(), Duration::from_secs(1));
        assert_eq!(b.next_delay(), Duration::from_secs(2));
        assert_eq!(b.next_delay(), Duration::from_secs(4));
        assert_eq!(b.next_delay(), MAX_BACKOFF);
        assert_eq!(b.next_delay(), MAX_BACKOFF, "no crece más allá del techo");
    }

    #[test]
    fn backoff_starts_over_after_a_healthy_run() {
        let mut b = Backoff::default();
        b.next_delay();
        b.next_delay();
        b.reset();
        assert_eq!(b.next_delay(), Duration::from_millis(500));
    }

    #[test]
    fn a_live_sidecar_that_keeps_talking_is_left_alone() {
        let mut w = Watchdog::new(TIMEOUT);
        assert_eq!(w.tick(Duration::from_secs(5), true), None);
    }

    #[test]
    fn a_dead_process_is_restarted_immediately() {
        let mut w = Watchdog::new(TIMEOUT);
        assert_eq!(w.tick(Duration::from_secs(1), false), Some(Duration::ZERO));
    }

    #[test]
    fn a_silent_but_live_process_is_restarted_too() {
        // El caso feo: el proceso existe, así que `try_wait` dice que está
        // vivo, pero hace rato que no manda señales. Un cuelgue de CUDA se ve
        // exactamente así.
        let mut w = Watchdog::new(TIMEOUT);
        assert_eq!(w.tick(Duration::from_secs(3), true), None);
        assert_eq!(w.tick(Duration::from_secs(4), true), Some(Duration::ZERO));
    }

    #[test]
    fn a_message_resets_the_silence_counter() {
        let mut w = Watchdog::new(TIMEOUT);
        w.tick(Duration::from_secs(5), true);
        w.on_message();
        assert_eq!(w.tick(Duration::from_secs(5), true), None);
    }

    #[test]
    fn repeated_restarts_wait_longer_each_time() {
        let mut w = Watchdog::new(TIMEOUT);
        assert_eq!(w.tick(Duration::from_secs(1), false), Some(Duration::ZERO));
        assert_eq!(
            w.tick(Duration::from_secs(1), false),
            Some(Duration::from_millis(500))
        );
        assert_eq!(
            w.tick(Duration::from_secs(1), false),
            Some(Duration::from_secs(1))
        );
    }

    #[test]
    fn a_sidecar_that_recovers_gets_a_clean_slate() {
        let mut w = Watchdog::new(TIMEOUT);
        w.tick(Duration::from_secs(1), false);
        w.tick(Duration::from_secs(1), false);
        w.on_message(); // el sidecar nuevo saludó
        assert_eq!(w.tick(Duration::from_secs(1), false), Some(Duration::ZERO));
    }
}
