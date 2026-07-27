import { writable } from "svelte/store";

/// Espeja `DictationState` del backend (`src-tauri/src/hotkeys.rs`).
/// Los nombres tienen que coincidir literalmente: el evento `state-changed`
/// manda la variante serializada tal cual. Del lado de Rust hay un test
/// (`the_serialized_names_match_the_frontend_union`) que lo verifica.
export type Status = "Idle" | "Listening" | "Transcribing";

export const status = writable<Status>("Idle");

/// Texto para la píldora. Vive acá y no en el componente para poder testearlo
/// sin montar Svelte.
export function statusLabel(status: Status): string {
  switch (status) {
    case "Listening":
      return "Escuchando";
    case "Transcribing":
      return "Transcribiendo";
    case "Idle":
      return "Dictado apagado";
  }
}

/// ¿La píldora tiene que animar las ondas?
///
/// Sí mientras haya una sesión abierta, incluido `Transcribing`: el micrófono
/// sigue grabando mientras el motor procesa el segmento anterior, y congelar la
/// animación ahí haría creer que dejó de escuchar.
export function isAnimated(status: Status): boolean {
  return status !== "Idle";
}
