//! Banco de frases español → japonés para pedir ayuda sin conexión.
//!
//! El banco viaja compilado dentro del binario (`include_str!`): no depende de
//! internet ni de un archivo en disco que pueda faltar o romperse en el
//! momento en que más hace falta. El mismo JSON alimenta la tarjeta imprimible
//! de `docs/phrasebook/` (ver `scripts/phrasebook-card.mjs`).
//!
//! La idea es que el pipeline, en un "modo frases", le pase a [`Phrasebook::find`]
//! cada segmento ya post-procesado y la UI muestre en grande el japonés de la
//! primera coincidencia, con las siguientes como alternativas. Ese cableado
//! todavía no existe; este módulo es sólo la búsqueda, pura y determinista.
//!
//! # Cómo se busca
//!
//! Cada frase tiene `triggers`, disparadores escritos en el JSON:
//!
//! - `"dolor cabeza"`: todas las palabras tienen que aparecer en lo dictado, en
//!   cualquier orden. Vale una palabra por punto, así que el disparador más
//!   específico le gana al genérico ("me duele la cabeza" → `dolor-cabeza` y no
//!   `dolor`).
//! - `"respir*"`: la palabra que termina en `*` es un prefijo ("respiro",
//!   "respirar"). Sirve para las conjugaciones sin listar cada una.
//! - `"=necesito ayuda"`: lo dictado tiene que ser exactamente eso. Para frases
//!   cortas y genéricas que, como palabra suelta, taparían a otras más útiles.
//!
//! Además, dictar el texto en español de la frase tal cual la encuentra
//! siempre, aunque no figure entre sus disparadores.
//!
//! Todo se compara normalizado: minúsculas, sin tildes ni `ñ`, sin puntuación.
//! Whisper a veces pone tildes y a veces no; el usuario no tiene por qué
//! enterarse.

use serde::Deserialize;
use std::sync::OnceLock;

const ES_JA_JSON: &str = include_str!("../phrasebook/es-ja.json");

/// Puntaje de una coincidencia con la frase entera: le gana a cualquier
/// combinación de palabras clave, que en la práctica no pasan de cuatro.
const WHOLE_UTTERANCE_SCORE: usize = 100;

#[derive(Debug, Deserialize)]
pub struct Phrasebook {
    pub categories: Vec<Category>,
    pub phrases: Vec<Phrase>,
}

#[derive(Debug, Deserialize)]
pub struct Category {
    pub id: String,
    pub es: String,
    pub ja: String,
}

#[derive(Debug, Deserialize)]
pub struct Phrase {
    pub id: String,
    pub category: String,
    /// 1 = crítica (va en la tarjeta de emergencia), 2 = frecuente, 3 = útil.
    /// A igual puntaje, gana la de prioridad más alta (número más chico).
    pub priority: u8,
    pub es: String,
    pub ja: String,
    /// Lectura en Hepburn, para quien quiera decirla en voz alta.
    pub romaji: String,
    #[serde(default)]
    pub triggers: Vec<String>,
}

/// Una frase encontrada y qué tan bien coincidió con lo dictado.
#[derive(Debug, Clone, Copy)]
pub struct Match<'a> {
    pub phrase: &'a Phrase,
    pub score: usize,
}

impl Phrasebook {
    /// El banco español → japonés que viene con la app.
    ///
    /// Se parsea una sola vez. El `expect` no es un riesgo en runtime: el JSON
    /// está dentro del binario y `bundled_phrasebook_is_consistent` lo valida
    /// en CI, así que si no parsea el build ya está rojo.
    pub fn es_ja() -> &'static Phrasebook {
        static BOOK: OnceLock<Phrasebook> = OnceLock::new();
        BOOK.get_or_init(|| {
            serde_json::from_str(ES_JA_JSON).expect("phrasebook/es-ja.json es JSON válido")
        })
    }

    /// Frases que coinciden con lo dictado, de la mejor a la peor.
    ///
    /// A igual puntaje decide la prioridad, y a igual prioridad el orden del
    /// archivo. Un texto sin palabras (silencio, sólo puntuación) no devuelve
    /// nada.
    pub fn find(&self, transcript: &str) -> Vec<Match<'_>> {
        let utterance = normalize(transcript);
        let tokens: Vec<&str> = utterance.split_whitespace().collect();
        if tokens.is_empty() {
            return Vec::new();
        }

        let mut matches: Vec<Match<'_>> = self
            .phrases
            .iter()
            .filter_map(|phrase| {
                let own_text =
                    (normalize(&phrase.es) == utterance).then_some(WHOLE_UTTERANCE_SCORE);
                phrase
                    .triggers
                    .iter()
                    .filter_map(|trigger| trigger_score(trigger, &utterance, &tokens))
                    .chain(own_text)
                    .max()
                    .map(|score| Match { phrase, score })
            })
            .collect();
        // `sort_by` es estable: a igual puntaje y prioridad queda el orden del
        // archivo, que es el que eligió quien armó el banco.
        matches.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then(a.phrase.priority.cmp(&b.phrase.priority))
        });
        matches
    }
}

/// Puntaje de un disparador contra lo dictado, o `None` si no coincide.
fn trigger_score(trigger: &str, utterance: &str, tokens: &[&str]) -> Option<usize> {
    if let Some(whole) = trigger.strip_prefix('=') {
        return (normalize(whole) == utterance).then_some(WHOLE_UTTERANCE_SCORE);
    }
    let words: Vec<&str> = trigger.split_whitespace().collect();
    if words.is_empty() {
        return None;
    }
    let all_present = words.iter().all(|word| match word.strip_suffix('*') {
        Some(stem) => {
            let stem = normalize(stem);
            tokens.iter().any(|token| token.starts_with(&stem))
        }
        None => tokens.contains(&normalize(word).as_str()),
    });
    all_present.then_some(words.len())
}

/// Minúsculas, sin tildes, `ñ` → `n`, y todo lo que no es letra o dígito pasa
/// a ser un espacio (colapsado). "¿Dónde está el BAÑO?" → "donde esta el bano".
pub fn normalize(text: &str) -> String {
    let folded: String = text
        .chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            c if c.is_alphanumeric() => c,
            _ => ' ',
        })
        .collect();
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    fn book() -> &'static Phrasebook {
        Phrasebook::es_ja()
    }

    fn best(transcript: &str) -> Option<&'static str> {
        book()
            .find(transcript)
            .first()
            .map(|m| m.phrase.id.as_str())
    }

    fn phrase(id: &str, priority: u8, es: &str, triggers: &[&str]) -> Phrase {
        Phrase {
            id: id.into(),
            category: "c".into(),
            priority,
            es: es.into(),
            ja: "ja".into(),
            romaji: "romaji".into(),
            triggers: triggers.iter().map(|t| t.to_string()).collect(),
        }
    }

    fn book_of(phrases: Vec<Phrase>) -> Phrasebook {
        Phrasebook {
            categories: vec![],
            phrases,
        }
    }

    // --- normalize -------------------------------------------------------

    #[test]
    fn normalize_folds_case_accents_enie_and_punctuation() {
        assert_eq!(normalize("¿Dónde está el BAÑO?"), "donde esta el bano");
        assert_eq!(normalize("  ¡Auxilio!!  ¡Ayúdenme! "), "auxilio ayudenme");
        assert_eq!(normalize("Wi-Fi"), "wi fi");
    }

    #[test]
    fn normalize_keeps_digits() {
        assert_eq!(normalize("Llame al 119."), "llame al 119");
    }

    #[test]
    fn normalize_of_only_punctuation_is_empty() {
        assert_eq!(normalize(" ¿...? "), "");
    }

    // --- find: reglas de coincidencia -------------------------------------

    #[test]
    fn find_requires_every_word_of_a_trigger_in_any_order() {
        let b = book_of(vec![phrase("p", 2, "x", &["dolor cabeza"])]);
        assert_eq!(b.find("la cabeza me da dolor").len(), 1);
        assert!(b.find("tengo dolor").is_empty());
    }

    #[test]
    fn find_prefers_the_more_specific_trigger() {
        let b = book_of(vec![
            phrase("generic", 1, "x", &["dolor"]),
            phrase("specific", 2, "y", &["dolor cabeza"]),
        ]);
        let found = b.find("tengo dolor de cabeza");
        assert_eq!(found[0].phrase.id, "specific");
        assert_eq!(found[1].phrase.id, "generic");
    }

    #[test]
    fn find_matches_star_words_as_prefixes_only() {
        let b = book_of(vec![phrase("p", 2, "x", &["respir*"])]);
        assert_eq!(b.find("no puedo respirar").len(), 1);
        assert_eq!(b.find("respiro mal").len(), 1);
        // El prefijo tiene que estar al comienzo de la palabra: "irrespirable"
        // contiene "respir", pero no empieza con él.
        assert!(b.find("el aire es irrespirable").is_empty());
    }

    #[test]
    fn find_does_not_match_a_plain_word_inside_another() {
        let b = book_of(vec![phrase("p", 2, "x", &["pie"])]);
        assert!(b.find("se me cayó la piel").is_empty());
    }

    #[test]
    fn find_whole_utterance_trigger_needs_the_exact_text() {
        let b = book_of(vec![phrase("p", 2, "x", &["=necesito ayuda"])]);
        assert_eq!(b.find("¡Necesito ayuda!").len(), 1);
        assert!(b.find("necesito ayuda para ir al baño").is_empty());
    }

    #[test]
    fn find_matches_the_phrase_own_spanish_text() {
        let b = book_of(vec![phrase("p", 2, "No.", &[])]);
        assert_eq!(b.find("no").len(), 1);
        assert!(b.find("no puedo").is_empty());
    }

    #[test]
    fn find_breaks_ties_by_priority_then_file_order() {
        let b = book_of(vec![
            phrase("low", 3, "x", &["agua"]),
            phrase("high-a", 1, "y", &["agua"]),
            phrase("high-b", 1, "z", &["agua"]),
        ]);
        let ids: Vec<&str> = b
            .find("agua")
            .iter()
            .map(|m| m.phrase.id.as_str())
            .collect();
        assert_eq!(ids, ["high-a", "high-b", "low"]);
    }

    #[test]
    fn find_returns_nothing_for_silence_or_unrelated_speech() {
        assert!(book().find("").is_empty());
        assert!(book().find(" ... ").is_empty());
        assert!(book().find("xyzzy qwerty").is_empty());
    }

    // --- el banco que viaja con la app --------------------------------------

    #[test]
    fn bundled_phrasebook_is_consistent() {
        let b = book();
        assert!(!b.phrases.is_empty());

        let categories: HashSet<&str> = b.categories.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(
            categories.len(),
            b.categories.len(),
            "ids de categoría repetidos"
        );

        let mut ids = HashSet::new();
        for p in &b.phrases {
            assert!(ids.insert(p.id.as_str()), "id repetido: {}", p.id);
            assert!(
                categories.contains(p.category.as_str()),
                "{}: categoría inexistente {}",
                p.id,
                p.category
            );
            assert!(
                (1..=3).contains(&p.priority),
                "{}: prioridad fuera de rango",
                p.id
            );
            for (field, value) in [("es", &p.es), ("ja", &p.ja), ("romaji", &p.romaji)] {
                assert!(!value.trim().is_empty(), "{}: `{field}` vacío", p.id);
            }
            for trigger in &p.triggers {
                match trigger.strip_prefix('=') {
                    Some(whole) => assert!(
                        !normalize(whole).is_empty(),
                        "{}: disparador `=` vacío",
                        p.id
                    ),
                    None => {
                        assert!(!trigger.trim().is_empty(), "{}: disparador vacío", p.id);
                        // Cada palabra tiene que quedar como una sola palabra
                        // normalizada: "wi-fi" se partiría en dos y nunca
                        // coincidiría, y un `*` solo coincidiría con todo.
                        for word in trigger.split_whitespace() {
                            let stem = word.strip_suffix('*').unwrap_or(word);
                            assert_eq!(
                                normalize(stem).split_whitespace().count(),
                                1,
                                "{}: palabra de disparador inválida {word:?}",
                                p.id
                            );
                        }
                    }
                }
            }
        }

        for c in &b.categories {
            assert!(
                b.phrases.iter().any(|p| p.category == c.id),
                "categoría sin frases: {}",
                c.id
            );
        }
    }

    #[test]
    fn no_two_phrases_share_a_trigger() {
        // Un disparador compartido deja el resultado en manos del desempate,
        // y quien edita el banco no se entera. Mejor que falle acá.
        let mut owner: HashMap<String, &str> = HashMap::new();
        for p in &book().phrases {
            let own_text = format!("={}", normalize(&p.es));
            let triggers = p.triggers.iter().map(|t| canonical_trigger(t));
            for key in triggers.chain(std::iter::once(own_text)) {
                if let Some(other) = owner.insert(key.clone(), &p.id) {
                    assert_eq!(other, p.id, "{key:?} está en {other} y en {}", p.id);
                }
            }
        }
    }

    fn canonical_trigger(trigger: &str) -> String {
        match trigger.strip_prefix('=') {
            Some(whole) => format!("={}", normalize(whole)),
            None => trigger
                .split_whitespace()
                .map(|w| match w.strip_suffix('*') {
                    Some(stem) => format!("{}*", normalize(stem)),
                    None => normalize(w),
                })
                .collect::<Vec<_>>()
                .join(" "),
        }
    }

    #[test]
    fn every_phrase_is_found_first_by_its_own_spanish_text() {
        // Si alguien lee en voz alta la frase de la tarjeta, la app tiene que
        // mostrar esa misma frase y no otra.
        for p in &book().phrases {
            assert_eq!(best(&p.es), Some(p.id.as_str()), "dictado: {:?}", p.es);
        }
    }

    #[test]
    fn common_requests_find_the_right_phrase() {
        let cases = [
            ("Necesito ir al baño", "bano"),
            ("necesito ayuda para ir al baño", "bano"),
            ("¿Dónde hay un baño?", "bano-accesible"),
            ("Llame una ambulancia por favor", "ambulancia"),
            ("llamen al 119", "ambulancia"),
            ("¡Ayuda!", "socorro"),
            ("necesito ayuda", "ayuda"),
            ("me duele", "dolor"),
            ("me duele mucho la cabeza", "dolor-cabeza"),
            ("no puedo respirar", "respirar"),
            ("creo que es disreflexia", "disreflexia"),
            ("tengo la presión alta", "disreflexia"),
            ("tómeme la presión", "presion"),
            ("siénteme rápido", "sentarme"),
            ("revise la sonda", "revisar-sonda"),
            ("la bolsa de orina está llena", "vaciar-bolsa"),
            ("tengo mucho calor", "calor"),
            ("tengo frío", "frio"),
            ("tengo sed", "beber"),
            ("tengo una úlcera por presión", "ulcera"),
            ("se me cayó el brazo", "brazo"),
            ("me caí de la silla", "caida"),
            ("no me deje solo", "no-me-deje"),
            ("hay un terremoto", "evacuar"),
            ("¿dónde está el ascensor?", "ascensor"),
            ("presione el botón del ascensor", "boton-ascensor"),
            ("use el traductor del teléfono", "traductor"),
            ("necesito un taxi", "taxi"),
            ("no hablo japonés", "no-hablo-japones"),
            ("estoy mareado", "mareo"),
            ("muchas gracias", "gracias"),
            ("estoy bien, gracias", "estoy-bien"),
            ("no gracias", "estoy-bien"),
            ("sí", "si"),
            ("no", "no"),
            ("¡pare!", "pare"),
        ];
        for (said, expected) in cases {
            assert_eq!(best(said), Some(expected), "dictado: {said:?}");
        }
    }
}
