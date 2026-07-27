/// Parte una frase en palabras para mostrarlas como chips clickeables en el
/// popup de vocabulario.
export function tokenize(text: string): string[] {
  const trimmed = text.trim();
  // `"".split(/\s+/)` devuelve `[""]`, que en el popup sería un chip vacío e
  // inclickeable.
  return trimmed === "" ? [] : trimmed.split(/\s+/);
}

/// Puntuación que puede quedar pegada a una palabra dictada. Incluye la
/// apertura de interrogación y exclamación del español, y las comillas
/// angulares.
const PUNCTUATION = /^[¿¡"'«»(\[{.,;:!?)\]}]+|[¿¡"'«»(\[{.,;:!?)\]}]+$/g;

/// Saca la puntuación de los bordes de una palabra, dejando la palabra en sí.
///
/// Es lo que se guarda como forma "mal escuchada": si se guardara `yeison,` con
/// la coma, el reemplazo no volvería a aplicarse nunca, porque el
/// post-procesador busca por palabra completa.
///
/// Los guiones y apóstrofes internos se conservan: `front-end` y `O'Brien` son
/// una sola palabra, no dos.
export function stripPunctuation(token: string): string {
  return token.replace(PUNCTUATION, "");
}
