# Release process

See [ADR 0006](adr/0006-branching-and-release-flow.md) for why this flow
exists. This document is the runbook: the commands, in order.

## Branches at a glance

| Branch | Source | Target | Merge type |
|---|---|---|---|
| `task/*`, `feat/*`, `fix/*` | `develop` | `develop` | Squash |
| `release/vX.Y.Z` | `develop` | `main` | Merge commit (`--no-ff`) |
| `hotfix/vX.Y.Z` | `main` | `main`, then `develop` | Merge commit (`--no-ff`) |

`main` only ever moves by merging a `release/*` or `hotfix/*` branch. Nothing
is pushed to `main` directly.

## Cutting a release

1. From an up-to-date `develop`, cut the release branch:

   ```bash
   git checkout develop && git pull origin develop
   git checkout -b release/v0.2.0
   ```

2. Bump the version in all three manifests:

   ```bash
   pnpm version:set 0.2.0
   ```

3. Move the `[Unreleased]` section of `CHANGELOG.md` to a dated, versioned
   section:

   ```markdown
   ## [0.2.0] - 2026-08-04
   ```

   Leave a fresh empty `[Unreleased]` above it for whatever lands in `develop`
   next.

4. Commit and push:

   ```bash
   git add Cargo.toml package.json src-tauri/tauri.conf.json CHANGELOG.md
   git commit -m "chore(release): bump to 0.2.0"
   git push -u origin release/v0.2.0
   ```

   CI's `versioning` job now checks that the branch name, the three
   manifests, and the CHANGELOG section all agree — a mismatch fails the
   build instead of surfacing later as a bad tag.

5. Tag the release candidate and let the CI-verified build sit for manual
   testing:

   ```bash
   git tag v0.2.0-rc.1
   git push origin v0.2.0-rc.1
   ```

   This publishes a **prerelease** on GitHub (`release.yml` marks any tag
   containing `-rc.` as a prerelease).

6. **Run the manual verification checklist below on the Windows target
   machine, against this branch.** This is the step CI cannot do for you.

7. If verification finds a bug: fix it directly on `release/v0.2.0` (bug
   fixes only — no new features here), push, tag `v0.2.0-rc.2`, repeat step 6.

8. Once verification passes, open the PR `release/v0.2.0` → `main`. Merge with
   a **merge commit**, not squash (squashing breaks the back-merge — see
   ADR 0006).

9. Tag the merge commit on `main` and push the tag:

   ```bash
   git checkout main && git pull origin main
   git tag v0.2.0
   git push origin v0.2.0
   ```

   This triggers `release.yml`, which publishes the (non-prerelease) GitHub
   Release with the CHANGELOG section as its body.

10. Back-merge `main` into `develop` so the version bump and CHANGELOG closure
    make it back:

    ```bash
    git checkout develop && git pull origin develop
    git merge --no-ff main -m "chore: merge v0.2.0 back into develop"
    git push origin develop
    ```

11. Delete the release branch (locally and on the remote) once the back-merge
    is pushed.

## Manual verification checklist (Windows target machine)

Run on the `release/*` branch, after the version bump, before promoting to
`main`. See CONTRIBUTING.md § Manual verification for what each check
exercises.

- [ ] `pnpm tauri dev` opens the window; pill and popup render correctly in
      both light and dark mode.
- [ ] Microphone capture: run the `mic_probe` example, confirm RMS rises when
      you speak.
- [ ] Text injection: run the `inject_probe` example against Notepad, confirm
      the pasted text lands and the clipboard is restored afterward.
- [ ] GPU transcription: confirm `whisper-rs` logs show the CUDA backend, not
      CPU fallback.
- [ ] Sidecar crash recovery: kill the sidecar process from Task Manager while
      dictating, confirm the supervisor restarts it and dictation keeps
      working.
- [ ] Autostart at Windows login.
- [ ] Full end-to-end dictation: toggle on, speak a sentence with at least one
      custom-vocabulary term, toggle off, confirm the text landed correctly in
      a focused application.

## Hotfixing a published version

For a bug found in something already on `main` that can't wait for the next
regular release:

```bash
git checkout main && git pull origin main
git checkout -b hotfix/v0.2.1
# fix, bump with: pnpm version:set 0.2.1
# close a [0.2.1] section in CHANGELOG.md
git push -u origin hotfix/v0.2.1
```

Same manual-verification expectation as a `release/*` branch, scoped to the
fix. PR to `main` with a merge commit, tag `v0.2.1`, then back-merge `main`
into `develop` exactly as in steps 9–10 above.

## GitHub setup (manual, not versioned)

These are repository settings, not files — configured once by hand:

- **Default branch:** `develop`.
- **`main` branch protection:** require a pull request before merging; require
  the status checks `Versión y CHANGELOG`, `Rust (fmt, clippy, tests)`,
  `Lógica pura (sin toolkit gráfico)`, and `Frontend (vitest, svelte-check)` to
  pass; no force-push; no deletion.
- **`develop` branch protection:** require the same four status checks; no
  force-push.
