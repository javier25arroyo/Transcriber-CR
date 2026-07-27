fn main() {
    // `tauri_build::build()` lee `tauri.conf.json` y exige que exista el
    // frontend construido (`../dist`). Sólo corre cuando se compila la app de
    // verdad; con `--no-default-features` la librería de lógica pura se
    // compila y testea sin frontend ni toolkit gráfico.
    #[cfg(feature = "app")]
    tauri_build::build();
}
