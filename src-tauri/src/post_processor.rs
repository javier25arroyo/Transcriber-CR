//! Post-proceso del texto que devuelve el STT: aplica las correcciones del
//! vocabulario y une los segmentos que van llegando en una frase legible.

use crate::vocab_store::Replacement;

/// Aplica todas las correcciones del vocabulario, en orden.
pub fn apply(raw: &str, replacements: &[Replacement]) -> String {
    let mut out = raw.to_string();
    for rep in replacements {
        out = replace_whole_word_ci(&out, &rep.heard, &rep.correct);
    }
    out
}

/// Reemplazo por palabra completa, sin distinguir mayúsculas.
///
/// El recorrido es por caracteres y no por bytes a propósito: en español hay
/// acentos y `ó` ocupa dos bytes, así que indexar el texto original con
/// posiciones calculadas sobre su versión en minúsculas —que además puede
/// cambiar de largo— es un panic esperando pasar.
fn replace_whole_word_ci(text: &str, heard: &str, correct: &str) -> String {
    let needle: Vec<char> = heard.to_lowercase().chars().collect();
    if needle.is_empty() {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    let mut idx = 0;
    while idx < text.len() {
        if let Some(end) = match_at(text, idx, &needle) {
            let starts_word = idx == 0
                || !text[..idx]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_alphanumeric);
            let ends_word = end == text.len()
                || !text[end..]
                    .chars()
                    .next()
                    .is_some_and(char::is_alphanumeric);
            if starts_word && ends_word {
                out.push_str(correct);
                idx = end;
                continue;
            }
        }
        // Sin match acá: se copia un carácter y se sigue desde el próximo
        // límite válido.
        let c = text[idx..]
            .chars()
            .next()
            .expect("idx < text.len() y es límite de carácter");
        out.push(c);
        idx += c.len_utf8();
    }
    out
}

/// Si `needle` (ya en minúsculas) empieza en el byte `start` de `text`,
/// devuelve el byte donde termina el match en `text`.
fn match_at(text: &str, start: usize, needle: &[char]) -> Option<usize> {
    let mut chars = text[start..].char_indices();
    let mut matched = 0;
    let mut end = start;
    while matched < needle.len() {
        let (offset, c) = chars.next()?;
        // Una mayúscula puede bajar a más de un carácter (ejemplo clásico: "İ").
        for lowered in c.to_lowercase() {
            if needle.get(matched) != Some(&lowered) {
                return None;
            }
            matched += 1;
        }
        end = start + offset + c.len_utf8();
    }
    Some(end)
}

/// Une el segmento nuevo al texto ya dictado. Whisper devuelve cada segmento
/// con su propia puntuación pero sin saber qué vino antes: si la frase anterior
/// cerró, el segmento siguiente arranca en mayúscula.
pub fn join_segments(prev: &str, next: &str) -> String {
    let prev = prev.trim_end();
    let next = next.trim_start();
    if prev.is_empty() {
        return next.to_string();
    }
    if next.is_empty() {
        return prev.to_string();
    }
    let next = if prev.ends_with(['.', '!', '?']) {
        capitalize_first(next)
    } else {
        next.to_string()
    };
    format!("{prev} {next}")
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vocab_store::Replacement;

    fn r(h: &str, c: &str) -> Replacement {
        Replacement {
            heard: h.into(),
            correct: c.into(),
        }
    }

    #[test]
    fn apply_replaces_case_insensitive_whole_words() {
        let reps = vec![r("yeison", "JSON")];
        assert_eq!(apply("mandale un Yeison", &reps), "mandale un JSON");
    }

    #[test]
    fn apply_does_not_replace_inside_words() {
        let reps = vec![r("son", "SUN")];
        assert_eq!(apply("la persona", &reps), "la persona");
    }

    #[test]
    fn apply_replaces_every_occurrence() {
        let reps = vec![r("yeison", "JSON")];
        assert_eq!(apply("yeison y yeison", &reps), "JSON y JSON");
    }

    #[test]
    fn apply_matches_multi_word_terms() {
        let reps = vec![r("guit jab", "GitHub")];
        assert_eq!(apply("subilo a guit jab", &reps), "subilo a GitHub");
    }

    #[test]
    fn apply_handles_accents_and_uppercase_without_panicking() {
        // Los límites de palabra son de caracteres, no de bytes: "ó" ocupa dos
        // bytes y una mayúscula acentuada puede cambiar de largo al pasar a
        // minúscula. Cortar por byte acá es un panic esperando pasar.
        let reps = vec![r("canción", "track")];
        assert_eq!(apply("la CANCIÓN suena", &reps), "la track suena");
        assert_eq!(apply("las canciónes", &reps), "las canciónes");
    }

    #[test]
    fn apply_leaves_text_untouched_when_there_are_no_replacements() {
        assert_eq!(apply("hola mundo", &[]), "hola mundo");
    }

    #[test]
    fn apply_ignores_an_empty_heard_term() {
        // Un `heard` vacío haría match en todas partes; tiene que ser un no-op
        // y no un bucle infinito.
        let reps = vec![r("", "X")];
        assert_eq!(apply("hola", &reps), "hola");
    }

    #[test]
    fn join_segments_adds_space_and_capitalizes_sentence_start() {
        assert_eq!(join_segments("hola.", "que tal"), "hola. Que tal");
        assert_eq!(join_segments("hola", "mundo"), "hola mundo");
    }

    #[test]
    fn join_segments_handles_empty_sides() {
        assert_eq!(join_segments("", "hola"), "hola");
        assert_eq!(join_segments("hola", ""), "hola");
        assert_eq!(join_segments("", ""), "");
    }
}
