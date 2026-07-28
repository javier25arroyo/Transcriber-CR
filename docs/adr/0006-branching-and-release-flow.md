# ADR 0006: Branching and release flow

- **Status:** Accepted
- **Date:** 2026-07-28
- **Source:** [Release process runbook](../release-process.md)
- **Supersedes:** the workflow described in
  [SDLC docs design §6, §8](../superpowers/specs/2026-07-26-sdlc-docs-design.md#6-flujo-de-trabajo)
  ("one branch per task, PR to `main`"). That spec is a dated design document
  and stays untouched — `docs/superpowers/` is explicitly out of scope for any
  change (SDLC docs design §3) — this ADR is the record that its branching
  section no longer reflects how the repo works.

## Context

The repository has a single long-lived branch (`main`) and zero tags. CI
cannot verify the parts of this app that matter most: GPU/CUDA backend
selection, microphone capture, clipboard injection, and the always-on-top
window all require a real Windows machine and are checked by hand (see
CONTRIBUTING.md § Manual verification, `docs/progress.md`). With one branch,
that manual verification either happens on `main` after the fact or doesn't
happen before code lands there — there is no point where a candidate is frozen
for it.

The project version (`0.1.0`) is also duplicated by hand across
`Cargo.toml`, `package.json`, and `src-tauri/tauri.conf.json`, with nothing
checking that the three agree.

## Decision

Two long-lived branches, three short-lived branch types, and a version check
enforced by CI:

- **`develop`** — the default branch and the integration point for ongoing
  work. `task/*`, `feat/*`, and `fix/*` branches merge here by **squash**,
  same as today.
- **`main`** — published code only. Every commit on `main` corresponds to a
  tag `vX.Y.Z`.
- **`release/vX.Y.Z`** — cut from `develop` when a version is ready to
  stabilize. This branch is where the version bump, the closed CHANGELOG
  section, and manual verification on Windows happen; the manual checklist in
  `docs/release-process.md` runs against this frozen candidate, not against
  `develop`. Merges to `main` with a merge commit (`--no-ff`), tagged
  `vX.Y.Z-rc.N` while under test and `vX.Y.Z` on the commit that lands in
  `main`.
- **`hotfix/vX.Y.Z`** — cut from `main` for an urgent fix to something already
  published. Merges to `main` the same way, then back-merges to `develop`.

`release/*` and `hotfix/*` merge to `main` with a **merge commit, not a
squash**: squashing here would rewrite the SHAs, so the back-merge to
`develop` would reintroduce the same changes as new commits and every future
release would carry avoidable conflicts.

CI (`.github/workflows/ci.yml`, job `versioning`) enforces two things that used
to be undocumented convention:
1. `Cargo.toml`, `package.json`, and `src-tauri/tauri.conf.json` report the
   same version (`scripts/version.mjs --check`).
2. On a `release/*` or `hotfix/*` branch, that version matches the branch name
   and `CHANGELOG.md` already has a closed section for it (not just
   `[Unreleased]`) — the "step before the tag" made mechanically checkable
   instead of relying on remembering to do it.

Pushing a tag `v*` triggers `.github/workflows/release.yml`, which re-verifies
the version against the tag and publishes a GitHub Release with the matching
CHANGELOG section as its body (`--prerelease` for `-rc.N` tags). No binaries
are attached — the v1 product spec explicitly excludes a distributable
installer.

## Consequences

- `main` only ever contains versions that went through the Windows manual
  checklist on their `release/*` branch. `develop` can move faster because it
  no longer carries that expectation.
- A CI check now fails a `release/*` push that forgot to bump the version or
  close the CHANGELOG section, instead of that surfacing later as a confusing
  tag/manifest mismatch.
- Two more long-lived-ish branch types and two back-merges to remember
  (`release/*` → `main` → `develop`, `hotfix/*` → `main` → `develop`) — more
  process than the previous single-branch flow, justified by the manual
  verification gate above.
- `CONTRIBUTING.md` § Workflow, the PR template, and `README.md` needed to
  change to point PRs at `develop` instead of `main`. `docs/superpowers/`
  itself was left alone; this ADR is the pointer that supersedes it.

## Alternatives considered

- **Full GitFlow with a long-lived `qa` branch** between `develop` and
  `release/*`: more control over what's "verified but not yet cut," but adds a
  second permanent back-merge to maintain by hand. For a one-person project
  it's the kind of extra bookkeeping that tends to drift out of sync silently.
  Rejected.
- **Trunk-based, no `develop`** (`main` stays the integration branch, only
  `release/*` branches get cut off it): the simplest option, but it leaves
  `main` holding code that was never verified on Windows — exactly the problem
  this ADR exists to fix. Rejected.
- **`release-please`-style automated versioning** driven by conventional
  commits: removes the manual bump, but hands control of the CHANGELOG (today
  hand-written, in Spanish) to a tool and adds an external dependency to the
  release process. Deferred — revisit if the manual bump becomes a real
  bottleneck.
