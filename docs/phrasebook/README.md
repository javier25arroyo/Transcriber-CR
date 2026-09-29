# Banco de frases español → japonés

Frases para pedir ayuda y cubrir necesidades en Japón **sin internet**,
pensadas para una persona con tetraplejia (sin movilidad del cuello para
abajo): emergencias, disreflexia autonómica, sonda y bolsa de orina, golpe de
calor, reposicionamiento, silla de ruedas, transporte.

Hay una sola fuente de verdad y dos formas de usarla:

| Qué | Dónde | Estado |
|---|---|---|
| Banco (datos) | [`src-tauri/phrasebook/es-ja.json`](../../src-tauri/phrasebook/es-ja.json) | Hecho |
| Búsqueda por voz en la app | [`src-tauri/src/phrasebook.rs`](../../src-tauri/src/phrasebook.rs) | Lógica hecha y testeada; falta el cableado (ver abajo) |
| Tarjeta offline / imprimible | [`tarjeta-es-ja.html`](tarjeta-es-ja.html) | Hecha, se usa hoy |

## La tarjeta

`tarjeta-es-ja.html` es un archivo único, sin nada externo: se abre sin
conexión en el navegador del teléfono o de la laptop.

- **En el teléfono:** tocar una frase la muestra en pantalla completa, en
  letra grande, para que la otra persona la lea. Si el teléfono tiene una voz
  japonesa instalada, aparece un botón para escucharla (en iOS: Ajustes →
  Accesibilidad → Contenido leído → Voces → Japonés; en Android: Ajustes de
  "Texto a voz" → descargar japonés). Instalarla **antes** del viaje: la
  descarga necesita internet.
- **En papel:** imprimir las primeras 3 hojas (portada, números de emergencia,
  datos médicos y frases críticas) y plastificarlas para llevarlas en la silla.
  "Más frases" arranca en hoja nueva para poder imprimirla aparte. Llenar a
  mano la sección de datos médicos: varias frases remiten a "esta tarjeta".

## Cómo busca la app

Cuando exista el "modo frases", cada segmento dictado se le pasa a
`Phrasebook::find` y la UI muestra el japonés de la mejor coincidencia. La
búsqueda es por palabras clave (`triggers` en el JSON), tolerante a tildes,
mayúsculas y puntuación; las reglas están documentadas al principio de
`phrasebook.rs`.

Lo que **falta** para usarla desde la app, en orden:

1. El motor real de Whisper en el sidecar (Task 6 del plan) y el cableado del
   pipeline (Task 11): sin eso la app no transcribe nada todavía.
2. Un modo frases que, en lugar de pegar el texto en la ventana con foco,
   muestre la frase en japonés en pantalla completa.
3. Decidir si la app habla en voz alta. El spec lo excluye hoy ("Sin TTS");
   Windows trae voces japonesas offline (hay que instalar el paquete de voz
   japonés antes), pero es un cambio de alcance.

## Editar el banco

1. Editar `src-tauri/phrasebook/es-ja.json`. Cada frase tiene `id`,
   `category`, `priority` (1 = va en las hojas de emergencia, 2 = frecuente,
   3 = útil), `es`, `ja`, `romaji` y `triggers`.
2. `cargo test -p voice-type --no-default-features phrasebook` — valida el
   JSON (ids únicos, categorías, disparadores sin choques) y que las frases
   comunes encuentren la entrada correcta.
3. `pnpm phrasebook:card` — regenera la tarjeta. El CI falla si quedó
   desactualizada (`pnpm phrasebook:check`).

## Antes de viajar

- **Que un hablante nativo revise el japonés**, en especial las frases
  médicas. Están escritas con cuidado y los términos (自律神経過反射,
  頸髄損傷, 導尿, 褥瘡, 熱中症) son los que usa la literatura médica japonesa,
  pero ningún banco de frases reemplaza esa revisión.
- Confirmar con tu médico el protocolo de disreflexia y ajustar las frases a
  tu caso (nivel de lesión, si usás sonda permanente o cateterismo
  intermitente, si sentís dolor debajo de la lesión).
- Los números de emergencia se verificaron en septiembre de 2026 (fuentes en
  `scripts/phrasebook-card.mjs`). La interpretación al español del 119 está
  confirmada en Tokio; en otras ciudades depende del municipio.
