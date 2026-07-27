//! Vocabulario personalizado: términos que sesgan el reconocimiento y
//! reemplazos deterministas que arreglan lo que Whisper igual escucha mal.

use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Vocab {
    pub terms: Vec<String>,
    pub replacements: Vec<Replacement>,
}

/// Una corrección: lo que Whisper transcribió (`heard`) y lo que el usuario
/// quería que dijera (`correct`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replacement {
    pub heard: String,
    pub correct: String,
}

impl Vocab {
    /// Lee el vocabulario del disco. Un archivo ausente, ilegible o corrupto
    /// devuelve el vocabulario vacío: quedarse sin correcciones es molesto,
    /// pero que la app no arranque por un JSON roto es peor.
    pub fn load(path: &Path) -> Vocab {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self)?)
    }

    /// Agrega una corrección.
    ///
    /// `terms` acumula formas correctas únicas: es lo que se le pasa a Whisper
    /// como sesgo, y repetir un término ahí no aporta nada.
    ///
    /// `replacements` está indexado por lo que se escuchó: un mismo `heard` no
    /// puede apuntar a dos formas correctas a la vez, así que corregir de nuevo
    /// la misma palabra pisa el destino anterior en lugar de acumular una
    /// entrada muerta. Varias formas mal escuchadas de un mismo término sí
    /// conviven (`"yeison"` y `"llason"` → `"JSON"`), que es el caso real.
    ///
    /// La comparación de `heard` es case-insensitive porque el reemplazo que la
    /// consume (`post_processor::apply`) también lo es: si acá se guardaran
    /// "Yeison" y "yeison" por separado, el segundo nunca llegaría a aplicarse.
    pub fn add(&mut self, heard: &str, correct: &str) {
        if !self.terms.iter().any(|t| t == correct) {
            self.terms.push(correct.to_string());
        }
        match self
            .replacements
            .iter_mut()
            .find(|r| r.heard.to_lowercase() == heard.to_lowercase())
        {
            Some(existing) => existing.correct = correct.to_string(),
            None => self.replacements.push(Replacement {
                heard: heard.to_string(),
                correct: correct.to_string(),
            }),
        }
    }

    /// Prompt inicial para whisper.cpp. No existe un parámetro formal de
    /// "hotwords": el sesgo se logra pasando los términos como contexto
    /// previo. Ver el spec §9 y la Task 6 del plan.
    pub fn initial_prompt(&self) -> String {
        self.terms.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn add_inserts_term_and_replacement_without_duplicates() {
        let mut v = Vocab::default();
        v.add("yeison", "JSON");
        v.add("yeison", "JSON"); // duplicado
        assert_eq!(v.terms, vec!["JSON".to_string()]);
        assert_eq!(v.replacements.len(), 1);
        assert_eq!(v.replacements[0].heard, "yeison");
        assert_eq!(v.replacements[0].correct, "JSON");
    }

    #[test]
    fn add_overwrites_the_target_of_a_known_heard_word() {
        let mut v = Vocab::default();
        v.add("yeison", "JSON");
        v.add("Yeison", "Jason"); // el usuario se corrige a sí mismo
        assert_eq!(v.replacements.len(), 1);
        assert_eq!(v.replacements[0].correct, "Jason");
    }

    #[test]
    fn add_keeps_several_mishearings_of_the_same_term() {
        let mut v = Vocab::default();
        v.add("yeison", "JSON");
        v.add("llason", "JSON");
        assert_eq!(v.terms, vec!["JSON".to_string()]);
        assert_eq!(v.replacements.len(), 2);
    }

    #[test]
    fn initial_prompt_lists_terms() {
        let mut v = Vocab::default();
        v.add("guit jab", "GitHub");
        v.add("rasp", "Rust");
        let p = v.initial_prompt();
        assert!(p.contains("GitHub"));
        assert!(p.contains("Rust"));
    }

    #[test]
    fn save_then_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("vocab.json");
        let mut v = Vocab::default();
        v.add("yeison", "JSON");
        v.save(&path).unwrap();
        let loaded = Vocab::load(&path);
        assert_eq!(loaded.terms, v.terms);
        assert_eq!(loaded.replacements.len(), 1);
    }

    #[test]
    fn load_missing_file_returns_default() {
        let v = Vocab::load(Path::new("no/existe/vocab.json"));
        assert!(v.terms.is_empty());
    }
}
