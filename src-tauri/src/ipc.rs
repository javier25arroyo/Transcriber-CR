//! Protocolo entre el proceso principal y el sidecar de STT.
//!
//! Un mensaje por línea, JSON, sobre stdin/stdout. Es el contrato compartido:
//! `stt-sidecar` depende de esta librería justamente para no duplicarlo (ver
//! [ADR 0001](../../docs/adr/0001-isolated-stt-sidecar.md)).

use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Mensajes del proceso principal hacia el sidecar.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToSidecar {
    /// Carga el modelo y lo deja caliente. Se reenvía tal cual después de cada
    /// reinicio del sidecar, y también cuando cambia el vocabulario (el
    /// `initial_prompt` es el que sesga el reconocimiento).
    LoadModel {
        model_path: String,
        language: String,
        initial_prompt: String,
    },
    /// Un segmento de audio ya normalizado a 16 kHz mono.
    ///
    /// Nota de tamaño: mandar las muestras como números en JSON es caro
    /// (un segmento de 5 s son 80 000 valores). Es aceptable porque el canal es
    /// una tubería local y el costo de serializar queda muy por debajo del de
    /// transcribir, pero si alguna vez aparece como cuello de botella, el
    /// cambio natural es un framing binario y merece su propio ADR.
    Transcribe { id: u64, pcm16k: Vec<f32> },
    /// Salida ordenada: el sidecar corta el loop y termina.
    Shutdown,
}

/// Mensajes del sidecar hacia el proceso principal.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FromSidecar {
    /// Modelo cargado; ya se le puede mandar audio.
    Ready,
    Result {
        id: u64,
        text: String,
    },
    /// Señal de vida periódica. El supervisor reinicia el sidecar si deja de
    /// llegar, aunque el proceso siga existiendo (por ejemplo, colgado).
    Heartbeat,
    Error {
        message: String,
    },
}

/// Serializa un mensaje como una línea JSON, con su salto final.
pub fn encode<T: Serialize>(msg: &T) -> String {
    // Un fallo acá sería un bug de los tipos de arriba, no una condición de
    // runtime: ninguno tiene mapas con claves no-string ni floats no
    // representables (el PCM viene de `f32` finitos).
    let mut s = serde_json::to_string(msg).expect("los mensajes de IPC son serializables");
    s.push('\n');
    s
}

/// Deserializa una línea recibida. Acepta el salto final, así que sirve tanto
/// con líneas ya recortadas como con la salida directa de [`encode`].
pub fn decode<T: DeserializeOwned>(line: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(line.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_sidecar_roundtrips() {
        let msg = ToSidecar::Transcribe {
            id: 7,
            pcm16k: vec![0.1, -0.2],
        };
        let line = encode(&msg);
        assert!(line.ends_with('\n'));
        let back: ToSidecar = decode(line.trim()).unwrap();
        match back {
            ToSidecar::Transcribe { id, pcm16k } => {
                assert_eq!(id, 7);
                assert_eq!(pcm16k.len(), 2);
            }
            other => panic!("variante incorrecta: {other:?}"),
        }
    }

    #[test]
    fn from_sidecar_result_roundtrips() {
        let msg = FromSidecar::Result {
            id: 3,
            text: "hola".into(),
        };
        let back: FromSidecar = decode(&encode(&msg)).unwrap();
        assert!(matches!(back, FromSidecar::Result { id: 3, .. }));
    }

    #[test]
    fn unit_variants_roundtrip() {
        for msg in [FromSidecar::Ready, FromSidecar::Heartbeat] {
            let back: FromSidecar = decode(&encode(&msg)).unwrap();
            assert_eq!(
                std::mem::discriminant(&back),
                std::mem::discriminant(&msg),
                "{msg:?} no sobrevivió el roundtrip"
            );
        }
        let back: ToSidecar = decode(&encode(&ToSidecar::Shutdown)).unwrap();
        assert!(matches!(back, ToSidecar::Shutdown));
    }

    #[test]
    fn encode_never_emits_an_embedded_newline() {
        // El framing es "un mensaje por línea": si un texto con salto de línea
        // se serializara crudo, el lector del otro lado partiría el mensaje al
        // medio. `serde_json` escapa el salto, y este test lo deja fijado.
        let line = encode(&FromSidecar::Result {
            id: 1,
            text: "primera\nsegunda".into(),
        });
        assert_eq!(line.matches('\n').count(), 1);
        assert!(line.ends_with('\n'));

        let back: FromSidecar = decode(line.trim()).unwrap();
        match back {
            FromSidecar::Result { text, .. } => assert_eq!(text, "primera\nsegunda"),
            other => panic!("variante incorrecta: {other:?}"),
        }
    }

    #[test]
    fn decode_rejects_garbage() {
        assert!(decode::<FromSidecar>("no soy json").is_err());
        assert!(decode::<FromSidecar>(r#"{"type":"Inventado"}"#).is_err());
    }
}
