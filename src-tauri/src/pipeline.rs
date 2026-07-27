//! El hilo conductor del dictado: qué pasa con cada segmento que devuelve el
//! motor de STT hasta que aparece como texto en la ventana enfocada.
//!
//! Acá vive la parte que decide *qué* se escribe. El cableado con el micrófono
//! y el sidecar (arrancar la captura, mandar los segmentos) necesita hardware y
//! se hace en la app; esto se testea con una función de inyección de mentira.

use crate::post_processor;
use crate::vocab_store::Vocab;

/// Estado de una sesión de dictado: el vocabulario, lo que se lleva dictado y a
/// dónde va el texto.
///
/// La inyección entra como closure para que el pipeline no sepa nada del
/// portapapeles: en la app real es `text_injector::inject`, en los tests es un
/// `String`.
pub struct Pipeline<'a> {
    vocab: Vocab,
    /// Todo lo dictado desde que se prendió el toggle. Sirve para dos cosas:
    /// saber si hay que capitalizar el segmento siguiente, y calcular qué
    /// pedazo es nuevo.
    transcript: String,
    /// El último segmento tal cual lo escuchó Whisper, sin correcciones.
    last_raw: String,
    inject: Box<dyn FnMut(&str) + 'a>,
}

impl<'a> Pipeline<'a> {
    pub fn new(vocab: Vocab, inject: impl FnMut(&str) + 'a) -> Self {
        Self {
            vocab,
            transcript: String::new(),
            last_raw: String::new(),
            inject: Box::new(inject),
        }
    }

    /// Procesa un segmento transcripto: corrige, une con lo anterior y escribe
    /// en la ventana enfocada **sólo el pedazo nuevo**.
    ///
    /// El texto se pega mientras la persona habla, así que reinyectar la frase
    /// entera en cada segmento la duplicaría en pantalla.
    pub fn on_result(&mut self, _id: u64, raw: String) {
        if is_non_speech(&raw) {
            return;
        }
        self.last_raw = raw.clone();

        let corrected = post_processor::apply(&raw, &self.vocab.replacements);
        let joined = post_processor::join_segments(&self.transcript, &corrected);
        let Some(new_text) = joined.strip_prefix(self.transcript.as_str()) else {
            // No debería pasar: `join_segments` conserva el prefijo porque el
            // transcript nunca termina en espacio. Si pasara, escribir todo de
            // nuevo duplicaría texto en el documento del usuario, así que se
            // prefiere no escribir nada.
            return;
        };
        if new_text.is_empty() {
            return;
        }

        let new_text = new_text.to_string();
        self.transcript = joined;
        (self.inject)(&new_text);
    }

    /// Cierra la sesión de dictado. El próximo segmento arranca una frase
    /// nueva: puede ir a otra ventana, y arrastrar el texto anterior haría que
    /// se capitalizara mal y que el "pedazo nuevo" se calculara contra algo que
    /// ya no está en pantalla.
    pub fn finish(&mut self) {
        self.transcript.clear();
    }

    /// Lo dictado en esta sesión, ya corregido.
    pub fn transcript(&self) -> &str {
        &self.transcript
    }

    /// El último segmento como lo escuchó Whisper. Es lo que muestra el popup
    /// de vocabulario: el usuario tiene que poder tocar la palabra *mal
    /// entendida*, y en el texto ya corregido esa palabra no aparece.
    pub fn last_raw_segment(&self) -> &str {
        &self.last_raw
    }

    /// Agrega una corrección al vocabulario y devuelve el `initial_prompt`
    /// nuevo, para que quien llama se lo reenvíe al sidecar en caliente.
    pub fn add_vocab_word(&mut self, heard: &str, correct: &str) -> String {
        self.vocab.add(heard, correct);
        self.vocab.initial_prompt()
    }

    pub fn vocab(&self) -> &Vocab {
        &self.vocab
    }
}

/// ¿El segmento es sólo una anotación de "esto no es habla"?
///
/// Whisper marca música, silencio y ruido con cosas como `[BLANK_AUDIO]`,
/// `(música)` o `[ Risas ]`. Pegar eso en el documento del usuario sería un
/// desastre. Se descarta únicamente cuando el segmento entero es la anotación:
/// una frase real que además tenga paréntesis se escribe como corresponde.
fn is_non_speech(segment: &str) -> bool {
    let t = segment.trim();
    if t.is_empty() {
        return true;
    }
    (t.starts_with('[') && t.ends_with(']')) || (t.starts_with('(') && t.ends_with(')'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    /// Arma un pipeline que escribe en un `String` compartido en vez de en la
    /// ventana enfocada.
    fn pipeline_with(vocab: Vocab) -> (Pipeline<'static>, Rc<RefCell<String>>) {
        let typed = Rc::new(RefCell::new(String::new()));
        let sink = Rc::clone(&typed);
        let pipeline = Pipeline::new(vocab, move |text| sink.borrow_mut().push_str(text));
        (pipeline, typed)
    }

    fn vocab_with(heard: &str, correct: &str) -> Vocab {
        let mut v = Vocab::default();
        v.add(heard, correct);
        v
    }

    #[test]
    fn on_result_applies_replacements_and_collects_text() {
        let (mut p, typed) = pipeline_with(vocab_with("yeison", "JSON"));

        p.on_result(1, "mandame un yeison".into());

        assert_eq!(*typed.borrow(), "mandame un JSON");
    }

    #[test]
    fn each_segment_types_only_what_is_new() {
        // Lo que se pega en la ventana es el pedazo nuevo, no la frase entera:
        // si se reinyectara todo el texto acumulado, cada segmento duplicaría
        // lo anterior en la pantalla.
        let (mut p, typed) = pipeline_with(Vocab::default());

        p.on_result(1, "hola.".into());
        p.on_result(2, "que tal".into());

        assert_eq!(*typed.borrow(), "hola. Que tal");
        assert_eq!(p.transcript(), "hola. Que tal");
    }

    #[test]
    fn segments_are_joined_with_a_space() {
        let (mut p, typed) = pipeline_with(Vocab::default());

        p.on_result(1, "hola".into());
        p.on_result(2, "mundo".into());

        assert_eq!(*typed.borrow(), "hola mundo");
    }

    #[test]
    fn an_empty_segment_types_nothing() {
        let (mut p, typed) = pipeline_with(Vocab::default());

        p.on_result(1, "hola".into());
        p.on_result(2, "   ".into());

        assert_eq!(*typed.borrow(), "hola");
    }

    #[test]
    fn non_speech_annotations_are_dropped() {
        // Whisper marca lo que no es habla con anotaciones entre corchetes o
        // paréntesis. Pegarlas en el documento del usuario sería un desastre.
        let (mut p, typed) = pipeline_with(Vocab::default());

        p.on_result(1, "[BLANK_AUDIO]".into());
        p.on_result(2, "(música)".into());
        p.on_result(3, "hola".into());
        p.on_result(4, "[ Risas ]".into());

        assert_eq!(*typed.borrow(), "hola");
    }

    #[test]
    fn a_sentence_that_mentions_brackets_is_not_dropped() {
        // El filtro es para el segmento que es *sólo* una anotación; una frase
        // real que además tenga paréntesis se escribe igual.
        let (mut p, typed) = pipeline_with(Vocab::default());

        p.on_result(1, "abrí el archivo (el nuevo)".into());

        assert_eq!(*typed.borrow(), "abrí el archivo (el nuevo)");
    }

    #[test]
    fn last_raw_segment_is_what_whisper_heard_not_what_was_typed() {
        // El popup de vocabulario le muestra al usuario la última frase para
        // que toque la palabra mal entendida. Tiene que ser el texto crudo: si
        // se le mostrara el texto ya corregido, la palabra a corregir no
        // estaría ahí.
        let (mut p, _typed) = pipeline_with(vocab_with("yeison", "JSON"));

        p.on_result(1, "mandame un yeison".into());

        assert_eq!(p.last_raw_segment(), "mandame un yeison");
    }

    #[test]
    fn adding_a_word_fixes_the_following_segments() {
        let (mut p, typed) = pipeline_with(Vocab::default());
        p.on_result(1, "un yeison".into());

        let prompt = p.add_vocab_word("yeison", "JSON");

        assert!(prompt.contains("JSON"), "el prompt sesga hacia el término");
        p.on_result(2, "otro yeison".into());
        assert_eq!(*typed.borrow(), "un yeison otro JSON");
    }

    #[test]
    fn finishing_starts_the_next_dictation_from_scratch() {
        // Al apagar el toggle se cierra la frase. El próximo dictado no puede
        // arrastrar el texto anterior: iría a otra ventana y capitalizaría mal.
        let (mut p, typed) = pipeline_with(Vocab::default());
        p.on_result(1, "hola.".into());
        p.finish();

        p.on_result(2, "que tal".into());

        assert_eq!(*typed.borrow(), "hola.que tal");
        assert_eq!(p.transcript(), "que tal");
    }
}
