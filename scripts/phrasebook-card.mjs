#!/usr/bin/env node
// Genera (o verifica) la tarjeta de frases español → japonés a partir del
// banco que usa la app: src-tauri/phrasebook/es-ja.json → docs/phrasebook/tarjeta-es-ja.html.
//
// La tarjeta es un HTML autocontenido: sin fuentes, scripts ni imágenes
// externas, así que se abre sin internet en el teléfono o la laptop y se
// imprime tal cual. Ver docs/phrasebook/README.md.
//
//   node scripts/phrasebook-card.mjs          # regenera la tarjeta
//   node scripts/phrasebook-card.mjs --check  # falla si quedó desactualizada
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const SOURCE = join(repoRoot, "src-tauri", "phrasebook", "es-ja.json");
const OUTPUT = join(repoRoot, "docs", "phrasebook", "tarjeta-es-ja.html");

// La frase que abre la tarjeta, en grande: lo primero que alguien tiene que
// entender antes de ayudar.
const HERO_ID = "tetraplejia";

// Números verificados en septiembre de 2026. Fuentes:
// - 119 con intérprete (inglés, chino, coreano, portugués y español) en Tokio:
//   https://www.hokeniryo.metro.tokyo.lg.jp/iryo/iryo_hoken/medical_info_eng/emergency_call
// - Japan Visitor Hotline (JNTO): https://www.japan.travel/en/plan/hotline/
const EMERGENCY_NUMBERS = [
  {
    number: "119",
    ja: "救急車・消防",
    es: "Ambulancia y bomberos. En Tokio hay intérprete de español: no cuelgue, tarda unos segundos en conectarse. Fuera de Tokio depende del municipio.",
  },
  { number: "110", ja: "警察", es: "Policía." },
  {
    number: "050-3816-2787",
    ja: "訪日外国人旅行者向けホットライン",
    es: "Japan Visitor Hotline (JNTO): 24 horas, en inglés, chino y coreano. No atiende en español.",
  },
];

// Casillas para llenar a mano: varias frases del banco remiten a "esta tarjeta".
const MEDICAL_FIELDS = [
  ["名前", "Nombre"],
  ["生年月日", "Fecha de nacimiento"],
  ["血液型", "Tipo de sangre"],
  ["損傷レベル", "Nivel de la lesión (p. ej. C4)"],
  ["服用中の薬", "Medicamentos que tomo"],
  ["アレルギー", "Alergias"],
  ["主治医", "Mi médico (país de origen)"],
  ["緊急連絡先", "Contacto de emergencia"],
  ["滞在先", "Hotel y dirección en Japón"],
  ["旅行保険", "Seguro de viaje (póliza y teléfono)"],
];

function escapeHtml(text) {
  return text
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function phraseButton(phrase, extraClass = "") {
  const cls = extraClass ? `phrase ${extraClass}` : "phrase";
  return [
    `<button type="button" class="${cls}" id="${escapeHtml(phrase.id)}">`,
    `<span class="ja" lang="ja">${escapeHtml(phrase.ja)}</span>`,
    `<span class="es">${escapeHtml(phrase.es)}</span>`,
    `<span class="romaji">${escapeHtml(phrase.romaji)}</span>`,
    `</button>`,
  ].join("");
}

function phraseSections(book, keep) {
  const out = [];
  for (const category of book.categories) {
    const phrases = book.phrases.filter((p) => p.category === category.id && keep(p));
    if (phrases.length === 0) continue;
    out.push(
      `<section class="category">`,
      `<h3>${escapeHtml(category.es)} <span lang="ja">${escapeHtml(category.ja)}</span></h3>`,
      `<div class="grid">`,
      ...phrases.map((p) => phraseButton(p)),
      `</div>`,
      `</section>`,
    );
  }
  return out.join("\n");
}

function render(book) {
  const hero = book.phrases.find((p) => p.id === HERO_ID);
  if (!hero) throw new Error(`${relative(repoRoot, SOURCE)}: falta la frase "${HERO_ID}"`);
  const critical = (p) => p.priority === 1 && p.id !== HERO_ID;
  const rest = (p) => p.priority > 1;

  return `<!doctype html>
<!-- Generado por scripts/phrasebook-card.mjs a partir de src-tauri/phrasebook/es-ja.json. No editar a mano. -->
<html lang="es">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Tarjeta de ayuda</title>
<style>
:root {
  --bg: #ffffff;
  --fg: #16181d;
  --muted: #5b6270;
  --line: #d9dce3;
  --card: #f5f6f8;
  --accent: #b42318;
  --accent-bg: #fef3f2;
}
@media screen and (prefers-color-scheme: dark) {
  :root {
    --bg: #111317;
    --fg: #eef0f4;
    --muted: #a3aab8;
    --line: #2c313b;
    --card: #1a1d23;
    --accent: #ff8a80;
    --accent-bg: #2a1614;
  }
}
* { box-sizing: border-box; }
html { -webkit-text-size-adjust: 100%; }
body {
  margin: 0 auto;
  max-width: 60rem;
  padding: 16px;
  background: var(--bg);
  color: var(--fg);
  font: 16px/1.45 system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
}
:lang(ja) {
  font-family: "Hiragino Sans", "Hiragino Kaku Gothic ProN", "Yu Gothic", YuGothic, Meiryo,
    "Noto Sans CJK JP", "Noto Sans JP", IPAGothic, sans-serif;
}
h2 { font-size: 1.15rem; margin: 1.6rem 0 0.6rem; }
h3 { font-size: 0.95rem; color: var(--muted); margin: 1.1rem 0 0.4rem; font-weight: 600; }
h3 span { margin-left: 0.4em; }
.cover { text-align: center; padding: 0.5rem 0 0.25rem; }
.cover .read { font-size: 1.6rem; font-weight: 700; margin: 0; }
.cover .read-es { color: var(--muted); margin: 0.1rem 0 0.8rem; }
.box { border: 2px solid var(--line); border-radius: 12px; padding: 0.6rem 0.9rem; margin: 0.9rem 0; }
.box h2 { margin-top: 0.2rem; }
.emergency { border-color: var(--accent); background: var(--accent-bg); }
.emergency ul { list-style: none; margin: 0; padding: 0; }
.emergency li { display: grid; grid-template-columns: 9.5rem 1fr; gap: 0.2rem 0.8rem; padding: 0.3rem 0; }
.emergency .num { font-size: 1.35rem; font-weight: 800; color: var(--accent); font-variant-numeric: tabular-nums; }
.emergency .what span { display: block; }
.medical dl { display: grid; grid-template-columns: minmax(9rem, 14rem) 1fr; margin: 0; }
.medical dt, .medical dd { border-bottom: 1px solid var(--line); padding: 0.55rem 0 0.2rem; margin: 0; }
.medical dt small { display: block; color: var(--muted); }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(17rem, 1fr)); gap: 0.5rem; }
.phrase {
  display: flex; flex-direction: column; gap: 0.15rem;
  width: 100%; text-align: left; font: inherit; color: inherit;
  background: var(--card); border: 1px solid var(--line); border-radius: 10px;
  padding: 0.55rem 0.75rem; cursor: pointer; break-inside: avoid;
}
.phrase:focus-visible { outline: 3px solid var(--accent); outline-offset: 2px; }
.phrase .ja { font-size: 1.3rem; line-height: 1.35; font-weight: 600; }
.phrase .es { font-size: 0.92rem; }
.phrase .romaji { font-size: 0.8rem; color: var(--muted); font-style: italic; }
.phrase.hero { text-align: center; align-items: center; border: 2px solid var(--accent); }
.phrase.hero .ja { font-size: 1.75rem; }
.phrase.hero .es { font-size: 1rem; }
.hint { color: var(--muted); font-size: 0.85rem; }
footer { margin-top: 2rem; color: var(--muted); font-size: 0.8rem; }
#zoom {
  position: fixed; inset: 0; z-index: 10;
  display: flex; flex-direction: column; justify-content: center; align-items: center; gap: 1.2rem;
  padding: 24px 16px; background: var(--bg); text-align: center; cursor: pointer;
}
#zoom[hidden] { display: none; }
#zoom .ja { font-size: clamp(2rem, 9vw, 4.5rem); line-height: 1.3; font-weight: 700; overflow-wrap: anywhere; }
#zoom .es { font-size: 1.1rem; color: var(--muted); }
#zoom .speak {
  font: inherit; font-size: 1.1rem; padding: 0.7rem 1.4rem; border-radius: 999px;
  border: 2px solid var(--accent); background: var(--accent-bg); color: var(--fg); cursor: pointer;
}
#zoom .close { color: var(--muted); font-size: 0.9rem; }
@media (max-width: 30rem) {
  .emergency li { grid-template-columns: 1fr; }
  .medical dl { grid-template-columns: 1fr; }
  .medical dd { min-height: 2rem; }
}
/* Impresa, la parte crítica (portada, números, datos médicos y frases
   críticas) tiene que caber en pocas hojas para plastificarla y llevarla en
   la silla; "Más frases" arranca en hoja nueva para poder imprimirla aparte. */
@page { size: A4; margin: 10mm; }
@media print {
  body { max-width: none; padding: 0; font-size: 9.5pt; }
  .hint, #zoom { display: none !important; }
  h2 { font-size: 11.5pt; margin: 0.7rem 0 0.3rem; }
  h3 { font-size: 9.5pt; margin: 0.5rem 0 0.2rem; }
  .cover .read { font-size: 16pt; }
  .cover .read-es { margin-bottom: 0.4rem; }
  .box { padding: 0.3rem 0.6rem; margin: 0.5rem 0; }
  .emergency li { grid-template-columns: 10.5rem 1fr; padding: 0.15rem 0; }
  .emergency .num { font-size: 14pt; white-space: nowrap; }
  /* Una casilla por renglón: a mano, "medicamentos" o "contacto" no caben en media hoja. */
  .medical dl { grid-template-columns: 11rem 1fr; }
  .medical dt, .medical dd { padding: 0.35rem 0 0.1rem; }
  .medical dt small { display: inline; margin-left: 0.4em; }
  .grid { gap: 0.3rem; }
  .phrase { background: none; padding: 0.25rem 0.45rem; gap: 0.05rem; }
  .phrase .ja { font-size: 12pt; }
  .phrase .es { font-size: 9pt; }
  .phrase .romaji { font-size: 8pt; }
  .phrase.hero .ja { font-size: 17pt; }
  .phrase.hero .es { font-size: 10pt; }
  .more { break-before: page; }
  h2, h3 { break-after: avoid; }
}
</style>
</head>
<body>
<header class="cover">
<p class="read" lang="ja">このカードを読んでください</p>
<p class="read-es">Por favor, lea esta tarjeta.</p>
${phraseButton(hero, "hero")}
</header>
<p class="hint">Toque una frase para verla en pantalla completa (y escucharla, si el teléfono tiene una voz japonesa instalada).</p>

<section class="box emergency">
<h2>Números de emergencia <span lang="ja">緊急電話</span></h2>
<ul>
${EMERGENCY_NUMBERS.map(
  (n) =>
    `<li><span class="num">${escapeHtml(n.number)}</span><span class="what"><span lang="ja">${escapeHtml(n.ja)}</span><span>${escapeHtml(n.es)}</span></span></li>`,
).join("\n")}
</ul>
</section>

<section class="box medical">
<h2>Mis datos médicos <span lang="ja">医療情報</span></h2>
<dl>
${MEDICAL_FIELDS.map(
  ([ja, es]) => `<dt><span lang="ja">${escapeHtml(ja)}</span><small>${escapeHtml(es)}</small></dt><dd></dd>`,
).join("\n")}
</dl>
</section>

<section class="critical">
<h2>Frases críticas <span lang="ja">緊急時のお願い</span></h2>
${phraseSections(book, critical)}
</section>

<section class="more">
<h2>Más frases <span lang="ja">その他のお願い</span></h2>
${phraseSections(book, rest)}
</section>

<footer>
<p>Pronunciación: ō y ū son vocales largas. Las frases médicas no reemplazan las indicaciones de su médico.</p>
</footer>

<div id="zoom" hidden role="dialog" aria-modal="true" aria-label="Frase en grande">
<p class="ja" lang="ja"></p>
<p class="es"></p>
<button type="button" class="speak" hidden>🔊 Escuchar en japonés</button>
<p class="close">Toque en cualquier lugar para cerrar</p>
</div>

<script>
(() => {
  const zoom = document.getElementById("zoom");
  const zoomJa = zoom.querySelector(".ja");
  const zoomEs = zoom.querySelector(".es");
  const speakButton = zoom.querySelector(".speak");
  const synth = "speechSynthesis" in window ? window.speechSynthesis : null;

  // Preferimos una voz local: una voz de red no funciona sin internet.
  function japaneseVoice() {
    if (!synth) return null;
    const voices = synth.getVoices().filter((v) => v.lang.replace("_", "-").toLowerCase().startsWith("ja"));
    return voices.find((v) => v.localService) || voices[0] || null;
  }

  function open(phrase) {
    zoomJa.textContent = phrase.querySelector(".ja").textContent;
    zoomEs.textContent = phrase.querySelector(".es").textContent;
    speakButton.hidden = !japaneseVoice();
    zoom.hidden = false;
  }

  function close() {
    if (synth) synth.cancel();
    zoom.hidden = true;
  }

  document.addEventListener("click", (event) => {
    const phrase = event.target.closest(".phrase");
    if (phrase) open(phrase);
  });

  zoom.addEventListener("click", (event) => {
    if (event.target.closest(".speak")) {
      const utterance = new SpeechSynthesisUtterance(zoomJa.textContent);
      utterance.lang = "ja-JP";
      const voice = japaneseVoice();
      if (voice) utterance.voice = voice;
      synth.cancel();
      synth.speak(utterance);
      return;
    }
    close();
  });

  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape" && !zoom.hidden) close();
  });

  // Las voces cargan de forma asíncrona en varios navegadores.
  if (synth) {
    synth.addEventListener("voiceschanged", () => {
      if (!zoom.hidden) speakButton.hidden = !japaneseVoice();
    });
  }
})();
</script>
</body>
</html>
`;
}

const book = JSON.parse(readFileSync(SOURCE, "utf8"));
const html = render(book);

if (process.argv.includes("--check")) {
  let current = "";
  try {
    current = readFileSync(OUTPUT, "utf8");
  } catch {
    // Sin archivo: cae en el mensaje de abajo.
  }
  if (current !== html) {
    console.error(
      `${relative(repoRoot, OUTPUT)} no coincide con ${relative(repoRoot, SOURCE)}. ` +
        "Regenerala con: pnpm phrasebook:card",
    );
    process.exit(1);
  }
  console.log(`${relative(repoRoot, OUTPUT)} está al día.`);
} else {
  writeFileSync(OUTPUT, html);
  console.log(`Escrita ${relative(repoRoot, OUTPUT)}.`);
}
