import { get } from "svelte/store";
import { expect, test } from "vitest";
import { isAnimated, status, statusLabel, type Status } from "./state";

test("el estado por defecto es Idle", () => {
  expect(get(status)).toBe("Idle");
});

test("el store acepta las tres variantes del backend", () => {
  status.set("Listening");
  expect(get(status)).toBe("Listening");
  status.set("Transcribing");
  expect(get(status)).toBe("Transcribing");
  status.set("Idle");
  expect(get(status)).toBe("Idle");
});

test("cada estado tiene su texto en la píldora", () => {
  expect(statusLabel("Idle")).toBe("Dictado apagado");
  expect(statusLabel("Listening")).toBe("Escuchando");
  expect(statusLabel("Transcribing")).toBe("Transcribiendo");
});

test("las ondas siguen animadas mientras se transcribe", () => {
  // El micrófono sigue grabando mientras el motor procesa el segmento
  // anterior; congelar la animación ahí haría creer que dejó de escuchar.
  expect(isAnimated("Idle")).toBe(false);
  expect(isAnimated("Listening")).toBe(true);
  expect(isAnimated("Transcribing")).toBe(true);
});

test("ningún estado se queda sin texto", () => {
  const all: Status[] = ["Idle", "Listening", "Transcribing"];
  for (const s of all) {
    expect(statusLabel(s)).toBeTruthy();
  }
});
