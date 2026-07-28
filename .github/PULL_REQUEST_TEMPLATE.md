## What changes

<!-- What does this PR do, and why? -->

## How this was verified

<!--
- For pure logic: which tests cover it (`cargo test -p voice-type ...` / `npx vitest run`)?
- For anything touching hardware/GPU/clipboard/injection/autostart: what did
  you do by hand to confirm it works? (See CONTRIBUTING.md → Manual verification.)
-->

## Checklist

- [ ] This PR targets `develop` (or `main`, only if it's a `release/*` or `hotfix/*` branch — see [docs/release-process.md](../docs/release-process.md))
- [ ] `cargo fmt --all -- --check` and `cargo clippy -p voice-type --all-targets -- -D warnings` pass locally
- [ ] `cargo test -p voice-type` and `npx vitest run` pass locally
- [ ] `CHANGELOG.md` updated under `[Unreleased]`
- [ ] If `README.md` changed, **both** the English and Spanish sections were updated
- [ ] If this introduces a decision that's expensive to reverse, an ADR was added under `docs/adr/`
