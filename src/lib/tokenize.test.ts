import { expect, test } from "vitest";
import { stripPunctuation, tokenize } from "./tokenize";

test("parte la frase en palabras", () => {
  expect(tokenize("mandame un yeison")).toEqual(["mandame", "un", "yeison"]);
});

test("los espacios de más no generan tokens vacíos", () => {
  expect(tokenize("  hola   mundo  ")).toEqual(["hola", "mundo"]);
});

test("una frase vacía no tiene tokens", () => {
  // Sin esto, `"".split(/\s+/)` devuelve [""] y el popup mostraría un chip
  // vacío e inclickeable.
  expect(tokenize("")).toEqual([]);
  expect(tokenize("   ")).toEqual([]);
});

test("la puntuación pegada se saca de la palabra a corregir", () => {
  // Si se guardara "yeison," como forma mal escuchada, el reemplazo no
  // volvería a aplicar nunca: el post-procesador busca por palabra completa.
  expect(stripPunctuation("yeison,")).toBe("yeison");
  expect(stripPunctuation("¿yeison?")).toBe("yeison");
  expect(stripPunctuation("«yeison».")).toBe("yeison");
});

test("stripPunctuation no toca los acentos ni la ñ", () => {
  expect(stripPunctuation("canción.")).toBe("canción");
  expect(stripPunctuation("¡mañana!")).toBe("mañana");
});

test("una palabra con guion o apóstrofe se conserva entera", () => {
  expect(stripPunctuation("front-end")).toBe("front-end");
  expect(stripPunctuation("O'Brien")).toBe("O'Brien");
});
