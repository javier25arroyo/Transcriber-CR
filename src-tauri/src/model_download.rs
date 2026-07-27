//! Dónde vive el modelo de Whisper y de dónde se baja la primera vez.
//!
//! El modelo no viene en el repositorio (son cientos de MB); se descarga en el
//! primer arranque. Acá está lo que se puede decidir sin tocar la red: las
//! rutas, la URL y la validación del nombre.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Repositorio oficial de los modelos ggml de whisper.cpp, en Hugging Face.
const MODEL_BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main";

/// Carpeta de modelos dentro del directorio de datos de la app.
const MODELS_DIR: &str = "models";

/// Dónde va a quedar el modelo `name`.
pub fn model_path(app_data: &Path, name: &str) -> PathBuf {
    app_data.join(MODELS_DIR).join(name)
}

/// Archivo temporal de la descarga, al lado del definitivo.
///
/// Se baja a `.part` y recién al terminar se renombra. Un corte de luz o de red
/// a mitad de camino deja un `.part` incompleto en vez de un `.bin` truncado
/// que, en el próximo arranque, parecería un modelo válido y haría fallar la
/// carga con un error del motor nativo imposible de interpretar.
pub fn partial_path(model_path: &Path) -> PathBuf {
    let mut name = model_path
        .file_name()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("modelo"));
    name.push(".part");
    model_path.with_file_name(name)
}

/// URL de descarga del modelo. Validar el nombre con [`is_valid_model_name`]
/// antes de armarla.
pub fn model_url(name: &str) -> String {
    format!("{MODEL_BASE_URL}/{name}")
}

/// ¿El nombre es un modelo ggml y nada más que eso?
///
/// `model_name` sale de `config.json`, que es un archivo que el usuario puede
/// editar a mano (y que podría llegar copiado de cualquier lado). Sin esta
/// validación, un nombre con barras o `..` escribiría la descarga fuera de la
/// carpeta de modelos y apuntaría la URL a otro lugar del repositorio.
pub fn is_valid_model_name(name: &str) -> bool {
    if name.is_empty() || name.contains("..") {
        return false;
    }
    if name.contains('/') || name.contains('\\') || name.contains(':') {
        return false;
    }
    // Los modelos ggml se llaman todos `ggml-<algo>.bin`.
    name.starts_with("ggml-") && name.ends_with(".bin") && name.len() > "ggml-.bin".len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn model_path_joins_models_dir() {
        let p = model_path(Path::new("C:/data"), "ggml-large-v3-q5_0.bin");
        assert!(
            p.ends_with("models/ggml-large-v3-q5_0.bin")
                || p.ends_with("models\\ggml-large-v3-q5_0.bin")
        );
    }

    #[test]
    fn the_partial_download_sits_next_to_the_final_file() {
        let final_path = model_path(Path::new("C:/data"), "ggml-base.bin");
        let partial = partial_path(&final_path);

        assert_eq!(partial.parent(), final_path.parent());
        assert_ne!(partial, final_path);
        assert!(partial.to_string_lossy().ends_with(".part"));
    }

    #[test]
    fn model_url_points_at_the_official_ggml_repo() {
        let url = model_url("ggml-large-v3-q5_0.bin");
        assert!(url.starts_with("https://"));
        assert!(url.contains("ggerganov/whisper.cpp"));
        assert!(url.ends_with("/ggml-large-v3-q5_0.bin"));
    }

    #[test]
    fn a_plain_model_name_is_accepted() {
        for name in [
            "ggml-large-v3-q5_0.bin",
            "ggml-base.bin",
            "ggml-medium.en-q8_0.bin",
        ] {
            assert!(is_valid_model_name(name), "debería aceptar {name}");
        }
    }

    #[test]
    fn a_name_with_a_path_in_it_is_rejected() {
        // `model_name` sale de config.json, que es un archivo que el usuario
        // puede editar a mano. Sin esta validación, un nombre como
        // "../../algo.bin" escribiría la descarga fuera de la carpeta de
        // modelos, y una URL con barras apuntaría a otro lado del repositorio.
        for name in [
            "../ggml-base.bin",
            "../../etc/passwd",
            "sub/ggml-base.bin",
            "sub\\ggml-base.bin",
            "/absoluto/ggml-base.bin",
            "C:/otro/ggml-base.bin",
        ] {
            assert!(!is_valid_model_name(name), "debería rechazar {name}");
        }
    }

    #[test]
    fn a_name_that_is_not_a_ggml_model_is_rejected() {
        for name in ["", "ggml-base", "notas.txt", ".bin", "..."] {
            assert!(!is_valid_model_name(name), "debería rechazar {name}");
        }
    }
}
