#!/usr/bin/env node
// Sincroniza (o verifica) la versión del proyecto entre las tres fuentes que
// hoy se editan a mano: Cargo.toml ([workspace.package]), package.json y
// src-tauri/tauri.conf.json. Ver docs/release-process.md.
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const CARGO_TOML = join(repoRoot, "Cargo.toml");
const PACKAGE_JSON = join(repoRoot, "package.json");
const TAURI_CONF = join(repoRoot, "src-tauri", "tauri.conf.json");

const SEMVER_RE = /^\d+\.\d+\.\d+$/;

function readCargoWorkspaceVersion(text) {
  const sectionStart = text.indexOf("[workspace.package]");
  if (sectionStart === -1) {
    throw new Error("Cargo.toml: no se encontró la sección [workspace.package]");
  }
  const match = text.slice(sectionStart).match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error('Cargo.toml: no se encontró "version" dentro de [workspace.package]');
  }
  return match[1];
}

function withCargoWorkspaceVersion(text, newVersion) {
  const sectionStart = text.indexOf("[workspace.package]");
  if (sectionStart === -1) {
    throw new Error("Cargo.toml: no se encontró la sección [workspace.package]");
  }
  const before = text.slice(0, sectionStart);
  const section = text.slice(sectionStart);
  const updated = section.replace(/^version\s*=\s*"([^"]+)"/m, `version = "${newVersion}"`);
  if (updated === section) {
    throw new Error('Cargo.toml: no se pudo reemplazar "version" dentro de [workspace.package]');
  }
  return before + updated;
}

function writeJsonVersion(path, newVersion) {
  const original = readFileSync(path, "utf8");
  const parsed = JSON.parse(original);
  parsed.version = newVersion;
  const trailingNewline = original.endsWith("\n") ? "\n" : "";
  writeFileSync(path, JSON.stringify(parsed, null, 2) + trailingNewline);
}

function readVersions() {
  const cargoText = readFileSync(CARGO_TOML, "utf8");
  const packageJson = JSON.parse(readFileSync(PACKAGE_JSON, "utf8"));
  const tauriConf = JSON.parse(readFileSync(TAURI_CONF, "utf8"));
  return {
    "Cargo.toml": readCargoWorkspaceVersion(cargoText),
    "package.json": packageJson.version,
    "src-tauri/tauri.conf.json": tauriConf.version,
  };
}

function check(expect) {
  const versions = readVersions();
  const values = Object.values(versions);
  const allMatch = values.every((v) => v === values[0]);

  if (!allMatch) {
    console.error("Las versiones no coinciden entre archivos:");
    for (const [source, version] of Object.entries(versions)) {
      console.error(`  ${source}: ${version}`);
    }
    process.exit(1);
  }

  const version = values[0];

  if (expect !== undefined && version !== expect) {
    console.error(`La versión ${version} no coincide con la esperada ${expect}`);
    process.exit(1);
  }

  console.log(version);
}

function set(newVersion) {
  if (!SEMVER_RE.test(newVersion)) {
    console.error(`"${newVersion}" no es una versión SemVer válida (esperado X.Y.Z)`);
    process.exit(1);
  }

  const cargoText = readFileSync(CARGO_TOML, "utf8");
  writeFileSync(CARGO_TOML, withCargoWorkspaceVersion(cargoText, newVersion));
  writeJsonVersion(PACKAGE_JSON, newVersion);
  writeJsonVersion(TAURI_CONF, newVersion);

  console.log(newVersion);
}

function usage() {
  console.error("Uso:");
  console.error("  node scripts/version.mjs --check");
  console.error("  node scripts/version.mjs --check --expect X.Y.Z");
  console.error("  node scripts/version.mjs X.Y.Z");
}

const args = process.argv.slice(2);

if (args.includes("--check")) {
  const expectIndex = args.indexOf("--expect");
  const expect = expectIndex !== -1 ? args[expectIndex + 1] : undefined;
  if (expectIndex !== -1 && expect === undefined) {
    usage();
    process.exit(1);
  }
  check(expect);
} else if (args.length === 1 && SEMVER_RE.test(args[0])) {
  set(args[0]);
} else {
  usage();
  process.exit(1);
}
