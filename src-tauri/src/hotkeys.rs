//! Estado del dictado y el atajo global que lo prende y lo apaga.
//!
//! El registro del atajo en sí lo hace la app (necesita Tauri); lo que se
//! decide acá es qué significa apretarlo.

use serde::{Deserialize, Serialize};

/// En qué anda el dictado. Se serializa tal cual al frontend en el evento
/// `state-changed`; los nombres coinciden con el tipo `Status` de
/// `src/lib/state.ts` y hay un test que lo verifica.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DictationState {
    /// El atajo está apagado.
    Idle,
    /// Micrófono abierto, esperando o juntando voz.
    Listening,
    /// Hay al menos un segmento en el motor de STT.
    Transcribing,
}

impl DictationState {
    /// Qué pasa al apretar el atajo.
    ///
    /// Apagado arranca; cualquier otro estado apaga. En particular
    /// `Transcribing` también apaga: para quien está dictando el toggle seguía
    /// prendido, y volver a arrancar la captura sería lo contrario de lo que
    /// pidió.
    pub fn toggled(self) -> Self {
        match self {
            DictationState::Idle => DictationState::Listening,
            _ => DictationState::Idle,
        }
    }

    /// ¿Hay una sesión de dictado abierta?
    pub fn is_dictating(self) -> bool {
        !matches!(self, DictationState::Idle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_flips_between_idle_and_listening() {
        let mut s = DictationState::Idle;
        s = s.toggled();
        assert_eq!(s, DictationState::Listening);
        s = s.toggled();
        assert_eq!(s, DictationState::Idle);
    }

    #[test]
    fn toggling_while_transcribing_stops_the_dictation() {
        // Apretar el atajo mientras se está transcribiendo tiene que parar,
        // no arrancar de nuevo: para el usuario el dictado seguía "prendido".
        assert_eq!(DictationState::Transcribing.toggled(), DictationState::Idle);
    }

    #[test]
    fn only_idle_means_not_dictating() {
        assert!(!DictationState::Idle.is_dictating());
        assert!(DictationState::Listening.is_dictating());
        assert!(DictationState::Transcribing.is_dictating());
    }

    #[test]
    fn the_serialized_names_match_the_frontend_union() {
        // `src/lib/state.ts` declara `type Status = "Idle" | "Listening" |
        // "Transcribing"` y el evento `state-changed` manda esta variante tal
        // cual. Si alguien renombra una variante acá, la píldora se queda
        // mostrando el estado viejo sin ningún error a la vista: este test es
        // el que avisa.
        for (state, expected) in [
            (DictationState::Idle, "\"Idle\""),
            (DictationState::Listening, "\"Listening\""),
            (DictationState::Transcribing, "\"Transcribing\""),
        ] {
            assert_eq!(serde_json::to_string(&state).unwrap(), expected);
        }
    }
}
