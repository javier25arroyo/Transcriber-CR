//! El sidecar de STT, partido en dos: el manejo del protocolo (acá, testeable
//! en cualquier máquina) y el motor de reconocimiento (detrás del trait
//! [`Engine`], que en producción es whisper.cpp con CUDA).
//!
//! Esa división es a propósito. El motor real necesita el CUDA Toolkit para
//! compilar y una GPU para correr, así que si el manejo de mensajes viviera
//! pegado a él no se podría probar en ningún CI. Con el trait, lo único que
//! queda sin cobertura automática es la llamada a whisper.cpp.

use voice_type::ipc::{FromSidecar, ToSidecar};

#[cfg(feature = "whisper")]
mod whisper_engine;
#[cfg(feature = "whisper")]
pub use whisper_engine::WhisperEngine;

/// El motor de reconocimiento, visto desde el protocolo.
pub trait Engine {
    /// Carga el modelo y lo deja caliente. Se vuelve a llamar cuando cambia el
    /// vocabulario, porque el `initial_prompt` es parte de la configuración del
    /// reconocimiento.
    fn load(
        &mut self,
        model_path: &str,
        language: &str,
        initial_prompt: &str,
    ) -> Result<(), String>;

    /// Transcribe un segmento de audio a 16 kHz mono.
    fn transcribe(&mut self, pcm16k: &[f32]) -> Result<String, String>;
}

/// Qué hacer después de atender un mensaje.
#[derive(Debug, PartialEq, Eq)]
pub enum Flow {
    /// Seguir escuchando la entrada.
    Continue,
    /// Salir del loop: llegó `Shutdown`.
    Stop,
}

/// Atiende un mensaje del proceso principal.
///
/// Devuelve lo que hay que contestar (puede ser nada) y si el loop sigue. No
/// escribe en stdout ni lee de stdin: eso queda en `main.rs`, que es la parte
/// que no se puede testear sin procesos.
pub fn handle(msg: ToSidecar, engine: &mut dyn Engine) -> (Option<FromSidecar>, Flow) {
    match msg {
        ToSidecar::LoadModel {
            model_path,
            language,
            initial_prompt,
        } => {
            let respuesta = match engine.load(&model_path, &language, &initial_prompt) {
                Ok(()) => FromSidecar::Ready,
                Err(message) => FromSidecar::Error { message },
            };
            (Some(respuesta), Flow::Continue)
        }

        ToSidecar::Transcribe { id, pcm16k } => {
            let respuesta = match engine.transcribe(&pcm16k) {
                Ok(text) => FromSidecar::Result { id, text },
                // El error lleva el mismo `id` en el mensaje para que del otro
                // lado se sepa qué segmento se perdió.
                Err(message) => FromSidecar::Error {
                    message: format!("segmento {id}: {message}"),
                },
            };
            (Some(respuesta), Flow::Continue)
        }

        ToSidecar::Shutdown => (None, Flow::Stop),
    }
}

/// Motor de reposo: el que hay mientras `whisper-rs` no esté compilado (feature
/// `whisper`, Task 6 del plan). Contesta un error claro en vez de fingir que
/// transcribe.
#[derive(Debug, Default)]
pub struct UnavailableEngine;

impl Engine for UnavailableEngine {
    fn load(&mut self, _: &str, _: &str, _: &str) -> Result<(), String> {
        Err(Self::MESSAGE.to_string())
    }

    fn transcribe(&mut self, _: &[f32]) -> Result<String, String> {
        Err(Self::MESSAGE.to_string())
    }
}

impl UnavailableEngine {
    const MESSAGE: &'static str =
        "sidecar compilado sin motor STT: falta la feature `whisper` (Task 6 del plan)";
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Motor de mentira: anota lo que le pidieron y contesta lo que se le diga.
    #[derive(Default)]
    struct FakeEngine {
        cargado: Option<(String, String, String)>,
        falla_al_cargar: bool,
        falla_al_transcribir: bool,
        recibio_muestras: usize,
    }

    impl Engine for FakeEngine {
        fn load(&mut self, model: &str, lang: &str, prompt: &str) -> Result<(), String> {
            if self.falla_al_cargar {
                return Err("sin VRAM".into());
            }
            self.cargado = Some((model.into(), lang.into(), prompt.into()));
            Ok(())
        }

        fn transcribe(&mut self, pcm: &[f32]) -> Result<String, String> {
            if self.falla_al_transcribir {
                return Err("el modelo no está cargado".into());
            }
            self.recibio_muestras = pcm.len();
            Ok("hola mundo".into())
        }
    }

    fn load_model() -> ToSidecar {
        ToSidecar::LoadModel {
            model_path: "modelos/ggml-base.bin".into(),
            language: "es".into(),
            initial_prompt: "JSON, GitHub".into(),
        }
    }

    #[test]
    fn loading_the_model_answers_ready() {
        let mut engine = FakeEngine::default();

        let (respuesta, flow) = handle(load_model(), &mut engine);

        assert!(matches!(respuesta, Some(FromSidecar::Ready)));
        assert_eq!(flow, Flow::Continue);
        let (model, lang, prompt) = engine.cargado.unwrap();
        assert_eq!(model, "modelos/ggml-base.bin");
        assert_eq!(lang, "es", "el idioma va fijo a español");
        assert_eq!(prompt, "JSON, GitHub", "el vocabulario llega al motor");
    }

    #[test]
    fn a_model_that_does_not_load_answers_error_and_keeps_the_loop_alive() {
        // Quedarse sin VRAM no puede matar al sidecar: el supervisor lo
        // reiniciaría en loop. Contesta el error y sigue escuchando.
        let mut engine = FakeEngine {
            falla_al_cargar: true,
            ..Default::default()
        };

        let (respuesta, flow) = handle(load_model(), &mut engine);

        match respuesta {
            Some(FromSidecar::Error { message }) => assert!(message.contains("sin VRAM")),
            otra => panic!("respuesta inesperada: {otra:?}"),
        }
        assert_eq!(flow, Flow::Continue);
    }

    #[test]
    fn transcribing_answers_the_text_with_the_same_id() {
        let mut engine = FakeEngine::default();

        let (respuesta, _) = handle(
            ToSidecar::Transcribe {
                id: 7,
                pcm16k: vec![0.0; 1600],
            },
            &mut engine,
        );

        match respuesta {
            Some(FromSidecar::Result { id, text }) => {
                assert_eq!(id, 7, "el id tiene que volver igual o se pierde el orden");
                assert_eq!(text, "hola mundo");
            }
            otra => panic!("respuesta inesperada: {otra:?}"),
        }
        assert_eq!(engine.recibio_muestras, 1600);
    }

    #[test]
    fn a_failed_segment_says_which_one_failed() {
        // Sin el id en el error, del otro lado no hay forma de saber qué
        // segmento se perdió.
        let mut engine = FakeEngine {
            falla_al_transcribir: true,
            ..Default::default()
        };

        let (respuesta, flow) = handle(
            ToSidecar::Transcribe {
                id: 42,
                pcm16k: vec![0.0; 10],
            },
            &mut engine,
        );

        match respuesta {
            Some(FromSidecar::Error { message }) => {
                assert!(message.contains("42"), "el error nombra el segmento");
                assert!(message.contains("el modelo no está cargado"));
            }
            otra => panic!("respuesta inesperada: {otra:?}"),
        }
        assert_eq!(flow, Flow::Continue);
    }

    #[test]
    fn shutdown_stops_the_loop_without_answering() {
        let mut engine = FakeEngine::default();

        let (respuesta, flow) = handle(ToSidecar::Shutdown, &mut engine);

        assert!(respuesta.is_none());
        assert_eq!(flow, Flow::Stop);
    }

    #[test]
    fn the_placeholder_engine_explains_itself() {
        let mut engine = UnavailableEngine;

        let (respuesta, flow) = handle(load_model(), &mut engine);

        match respuesta {
            Some(FromSidecar::Error { message }) => assert!(message.contains("whisper")),
            otra => panic!("respuesta inesperada: {otra:?}"),
        }
        assert_eq!(flow, Flow::Continue, "sigue escuchando igual");
    }
}
