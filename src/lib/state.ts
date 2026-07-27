import { writable } from "svelte/store";

/// Espeja `DictationState` del backend (`src-tauri/src/hotkeys.rs`, Task 12).
/// Los nombres tienen que coincidir literalmente: el evento `state-changed`
/// manda la variante serializada tal cual.
export type Status = "Idle" | "Listening" | "Transcribing";

export const status = writable<Status>("Idle");
