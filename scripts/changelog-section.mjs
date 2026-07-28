#!/usr/bin/env node
// Extrae de CHANGELOG.md la sección de una versión (o de [Unreleased] si la
// versión todavía no tiene una sección cerrada). Usado por el job de CI que
// valida releases y por el workflow que publica el GitHub Release con esas
// notas. Ver docs/release-process.md.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const CHANGELOG = join(repoRoot, "CHANGELOG.md");

function extractSection(text, heading) {
  const escaped = heading.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const headingRe = new RegExp(`^## \\[${escaped}\\].*$`, "m");
  const match = headingRe.exec(text);
  if (!match) return null;

  const start = match.index + match[0].length;
  const rest = text.slice(start);
  const nextHeadingIndex = rest.search(/^## \[/m);
  const body = nextHeadingIndex === -1 ? rest : rest.slice(0, nextHeadingIndex);
  return body.trim();
}

const raw = process.argv[2];
if (!raw) {
  console.error("Uso: node scripts/changelog-section.mjs X.Y.Z");
  process.exit(1);
}

const match = /^(\d+\.\d+\.\d+)(?:-rc\.\d+)?$/.exec(raw);
if (!match) {
  console.error(`"${raw}" no tiene forma de versión SemVer (X.Y.Z o X.Y.Z-rc.N)`);
  process.exit(1);
}
const baseVersion = match[1];

const text = readFileSync(CHANGELOG, "utf8");

let section = extractSection(text, baseVersion);
if (section === null) {
  section = extractSection(text, "Unreleased");
}

if (section === null || section.length === 0) {
  console.error(`No se encontró una sección para ${baseVersion} ni [Unreleased] en CHANGELOG.md`);
  process.exit(1);
}

console.log(section);
