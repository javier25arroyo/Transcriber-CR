//! Motor real de STT: whisper.cpp vía `whisper-rs`, detrás de la feature
//! `whisper` (Task 6 del plan).
//!
//! Se compila sin la feature `cuda` de whisper-rs: esta máquina tiene driver
//! NVIDIA pero no el CUDA Toolkit, así que usa el backend CPU de whisper.cpp
//! (ver el comentario en `Cargo.toml`). Prender CUDA más adelante no cambia
//! nada de este archivo — sólo la línea de la dependencia.
#![cfg(feature = "whisper")]

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use crate::Engine;

/// Envuelve el contexto de whisper.cpp. El contexto se mantiene caliente entre
/// segmentos; `load()` lo recrea porque el idioma y el `initial_prompt`
/// (vocabulario del usuario) son parte de la configuración del reconocimiento.
#[derive(Default)]
pub struct WhisperEngine {
    ctx: Option<WhisperContext>,
    language: String,
    initial_prompt: String,
}

impl Engine for WhisperEngine {
    fn load(
        &mut self,
        model_path: &str,
        language: &str,
        initial_prompt: &str,
    ) -> Result<(), String> {
        let ctx = WhisperContext::new_with_params(model_path, WhisperContextParameters::default())
            .map_err(|e| format!("no se pudo cargar el modelo «{model_path}»: {e}"))?;

        self.ctx = Some(ctx);
        self.language = language.to_string();
        self.initial_prompt = initial_prompt.to_string();
        Ok(())
    }

    fn transcribe(&mut self, pcm16k: &[f32]) -> Result<String, String> {
        let ctx = self
            .ctx
            .as_ref()
            .ok_or("el modelo no está cargado (falta LoadModel)")?;

        let mut state = ctx.create_state().map_err(|e| e.to_string())?;

        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(&self.language));
        if !self.initial_prompt.is_empty() {
            params.set_initial_prompt(&self.initial_prompt);
        }
        // Con más de un hilo el backend CPU aprovecha los núcleos disponibles;
        // sin esto whisper.cpp corre en uno solo y la latencia se dispara.
        params.set_n_threads(recommended_threads());
        // El sidecar habla por stdout con el protocolo IPC. Cualquier cosa que
        // whisper.cpp imprima ahí rompería el framing JSON, así que se apaga
        // todo lo decorativo; los logs de diagnóstico van a stderr, que el
        // supervisor hereda aparte.
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_print_special(false);

        state
            .full(params, pcm16k)
            .map_err(|e| format!("falló la transcripción: {e}"))?;

        let n = state.full_n_segments();
        let mut out = String::new();
        for i in 0..n {
            let segment = state
                .get_segment(i)
                .ok_or_else(|| format!("segmento {i} fuera de rango"))?;
            let text = segment.to_str_lossy().map_err(|e| e.to_string())?;
            out.push_str(&text);
        }
        Ok(out.trim().to_string())
    }
}

/// Cuántos hilos usar para el reconocimiento en CPU. Se deja uno libre para que
/// el resto del sistema (captura de audio, UI) no se quede sin núcleo, con un
/// piso de 1 por si `available_parallelism` no puede consultarlo.
fn recommended_threads() -> i32 {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    cores.saturating_sub(1).max(1) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_a_missing_model_returns_error_without_panicking() {
        let mut engine = WhisperEngine::default();

        let result = engine.load("no/existe/modelo.bin", "es", "");

        assert!(result.is_err());
    }

    #[test]
    fn transcribing_before_loading_a_model_is_an_error() {
        let mut engine = WhisperEngine::default();

        let result = engine.transcribe(&[0.0; 1600]);

        assert!(result.is_err());
    }

    #[test]
    fn recommends_at_least_one_thread() {
        assert!(recommended_threads() >= 1);
    }
}
