// Sin consola en Windows release: la app vive en la bandeja, no en una terminal.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    voice_type::run();
}
