//! Ajustes de la app: atajo, modelo, micrófono y posición de la píldora.

use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

/// `#[serde(default)]` a nivel de struct: un archivo de config escrito por una
/// versión anterior no tiene los campos que se agreguen después, y sin esto
/// serde falla al deserializar y `load` termina descartando *todos* los
/// ajustes del usuario por un campo que falta.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Atajo global tipo toggle, en la sintaxis de `tauri-plugin-global-shortcut`.
    pub toggle_shortcut: String,
    /// Nombre del archivo del modelo ggml dentro de la carpeta `models`.
    pub model_name: String,
    /// Micrófono elegido; `None` = el dispositivo de entrada por defecto.
    pub mic_device: Option<String>,
    /// Última posición de la píldora en pantalla.
    pub pill_pos: Option<(i32, i32)>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            toggle_shortcut: "Ctrl+Alt+Space".into(),
            model_name: "ggml-large-v3-q5_0.bin".into(),
            mic_device: None,
            pill_pos: None,
        }
    }
}

impl AppConfig {
    /// Igual que [`crate::vocab_store::Vocab::load`]: un archivo ausente o
    /// corrupto cae a los valores por defecto en vez de impedir el arranque.
    pub fn load(path: &Path) -> AppConfig {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_toggle_shortcut() {
        let c = AppConfig::default();
        assert_eq!(c.toggle_shortcut, "Ctrl+Alt+Space");
    }

    #[test]
    fn save_load_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let c = AppConfig {
            mic_device: Some("Mic USB".into()),
            pill_pos: Some((120, 640)),
            ..Default::default()
        };
        c.save(&path).unwrap();

        let loaded = AppConfig::load(&path);
        assert_eq!(loaded.mic_device.as_deref(), Some("Mic USB"));
        assert_eq!(loaded.pill_pos, Some((120, 640)));
    }

    #[test]
    fn load_missing_file_returns_default() {
        let c = AppConfig::load(Path::new("no/existe/config.json"));
        assert_eq!(c.toggle_shortcut, AppConfig::default().toggle_shortcut);
    }

    #[test]
    fn load_keeps_known_fields_when_the_file_predates_a_new_one() {
        // Un config viejo no tiene los campos que se agreguen después. Tiene
        // que cargarse igual, completando con los valores por defecto, en vez
        // de descartar los ajustes del usuario enteros.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, r#"{"mic_device":"Mic viejo"}"#).unwrap();

        let c = AppConfig::load(&path);
        assert_eq!(c.mic_device.as_deref(), Some("Mic viejo"));
        assert_eq!(c.toggle_shortcut, "Ctrl+Alt+Space");
    }

    #[test]
    fn load_corrupt_file_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        std::fs::write(&path, "{ esto no es json").unwrap();

        assert_eq!(AppConfig::load(&path).toggle_shortcut, "Ctrl+Alt+Space");
    }

    #[test]
    fn save_creates_missing_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("anidado/mas/config.json");
        AppConfig::default().save(&path).unwrap();
        assert!(path.exists());
    }
}
