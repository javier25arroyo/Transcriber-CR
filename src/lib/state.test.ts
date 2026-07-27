import { get } from "svelte/store";
import { expect, test } from "vitest";
import { status } from "./state";

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
