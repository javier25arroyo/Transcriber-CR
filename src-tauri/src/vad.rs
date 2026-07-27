//! Segmentación por energía: corta el audio en trozos que valga la pena
//! transcribir, para que el texto aparezca mientras se habla y no todo junto al
//! final.

/// Umbral de energía (RMS) a partir del cual un bloque cuenta como voz.
const SPEECH_THRESHOLD: f32 = 0.01;

/// Largo máximo de un segmento. Whisper trabaja con una ventana de 30 s; un
/// segmento más largo se transcribiría truncado, así que hablar sin pausas
/// fuerza un corte antes de llegar ahí en vez de hacer crecer el buffer sin
/// límite.
const MAX_SEGMENT_MS: usize = 15_000;

/// Acumula bloques de audio y devuelve un segmento cerrado cuando detecta
/// silencio suficiente después de que hubo voz.
pub struct Segmenter {
    threshold: f32,
    /// Muestras de silencio seguidas que hacen falta para cerrar.
    silence_needed: usize,
    max_samples: usize,
    buffer: Vec<f32>,
    trailing_silence: usize,
    has_speech: bool,
}

impl Segmenter {
    pub fn new(silence_ms: u32, sample_rate: u32) -> Self {
        let samples_per_ms = sample_rate as usize / 1000;
        Self {
            threshold: SPEECH_THRESHOLD,
            silence_needed: silence_ms as usize * samples_per_ms,
            max_samples: MAX_SEGMENT_MS * samples_per_ms,
            buffer: Vec::new(),
            trailing_silence: 0,
            has_speech: false,
        }
    }

    /// Agrega un bloque de audio. Devuelve el segmento si este bloque lo cierra.
    ///
    /// El silencio anterior a la primera voz se descarta: no aporta nada a la
    /// transcripción y sólo hace más lento el paso por el modelo. El silencio
    /// del final sí se conserva, porque le da a Whisper el contexto de que la
    /// frase terminó.
    pub fn push(&mut self, frame: &[f32]) -> Option<Vec<f32>> {
        if frame.is_empty() {
            return None;
        }

        if is_speech(frame, self.threshold) {
            self.has_speech = true;
            self.trailing_silence = 0;
            self.buffer.extend_from_slice(frame);
        } else if self.has_speech {
            self.trailing_silence += frame.len();
            self.buffer.extend_from_slice(frame);
            if self.trailing_silence >= self.silence_needed {
                return Some(self.take());
            }
        } else {
            return None;
        }

        if self.buffer.len() >= self.max_samples {
            return Some(self.take());
        }
        None
    }

    /// Cierra lo que haya quedado pendiente (el usuario apagó el toggle).
    pub fn flush(&mut self) -> Option<Vec<f32>> {
        if self.has_speech {
            Some(self.take())
        } else {
            None
        }
    }

    fn take(&mut self) -> Vec<f32> {
        self.trailing_silence = 0;
        self.has_speech = false;
        std::mem::take(&mut self.buffer)
    }
}

fn is_speech(frame: &[f32], threshold: f32) -> bool {
    let rms = (frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32).sqrt();
    rms >= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loud(n: usize) -> Vec<f32> {
        vec![0.3; n]
    }

    fn silence(n: usize) -> Vec<f32> {
        vec![0.0; n]
    }

    #[test]
    fn emits_segment_after_silence_following_speech() {
        // 16k, 300ms de silencio para cerrar.
        let mut seg = Segmenter::new(300, 16_000);
        assert!(seg.push(&loud(1600)).is_none()); // 100ms habla
                                                  // suficiente silencio para cerrar (>=300ms):
        let out = seg.push(&silence(6400)); // 400ms silencio
        assert!(out.is_some());
        assert!(out.unwrap().len() >= 1600);
    }

    #[test]
    fn flush_returns_pending_speech() {
        let mut seg = Segmenter::new(300, 16_000);
        seg.push(&loud(1600));
        assert!(seg.flush().is_some());
    }

    #[test]
    fn pure_silence_emits_nothing() {
        let mut seg = Segmenter::new(300, 16_000);
        assert!(seg.push(&silence(8000)).is_none());
        assert!(seg.flush().is_none());
    }

    #[test]
    fn short_silence_does_not_close_the_segment() {
        // Una pausa entre palabras no tiene que partir la frase.
        let mut seg = Segmenter::new(300, 16_000);
        seg.push(&loud(1600));
        assert!(seg.push(&silence(1600)).is_none()); // 100ms < 300ms
        seg.push(&loud(1600));
        let out = seg
            .push(&silence(6400))
            .expect("cierra al llegar el silencio largo");
        assert!(out.len() >= 1600 * 2 + 1600);
    }

    #[test]
    fn silence_before_speech_is_not_buffered() {
        // Lo que se manda a transcribir no debería arrancar con el silencio
        // previo a que la persona empiece a hablar.
        let mut seg = Segmenter::new(300, 16_000);
        seg.push(&silence(16_000)); // un segundo de nada
        seg.push(&loud(1600));
        let out = seg.flush().unwrap();
        assert_eq!(out.len(), 1600);
    }

    #[test]
    fn a_segment_is_cut_before_it_outgrows_the_model_window() {
        // Hablar sin pausas no puede hacer crecer el buffer para siempre:
        // Whisper trabaja con una ventana de 30 s, y un segmento más largo se
        // transcribiría truncado.
        let mut seg = Segmenter::new(300, 16_000);
        let mut emitted = None;
        for _ in 0..(MAX_SEGMENT_MS / 100 + 2) {
            if let Some(out) = seg.push(&loud(1600)) {
                emitted = Some(out);
                break;
            }
        }
        let out = emitted.expect("el segmento se corta solo al llegar al máximo");
        assert!(out.len() <= 16_000 * 30);
    }

    #[test]
    fn state_resets_after_emitting() {
        let mut seg = Segmenter::new(300, 16_000);
        seg.push(&loud(1600));
        seg.push(&silence(6400)).unwrap();
        assert!(
            seg.flush().is_none(),
            "no quedan restos del segmento anterior"
        );
    }

    #[test]
    fn an_empty_frame_is_ignored() {
        let mut seg = Segmenter::new(300, 16_000);
        assert!(seg.push(&[]).is_none());
        assert!(seg.flush().is_none());
    }
}
