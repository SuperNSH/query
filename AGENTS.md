# AGENTS.md

Hard rules for humans and coding agents in this repo.

- Comments are 1-2 lines and state a non-obvious constraint. Never narrate the next line.
- Docs read like a competent human wrote them: direct, concrete, no marketing filler.
- Dead code is deleted. No speculative abstractions.
- Rust gates: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- TypeScript gates, from `packages/nsh-query`: `nsh check`, `nsh build`, `nsh test`.
- Keep the two versions' semantics the same. A behavior change lands in both, with a test in both.
- Tests go through the UI on a manual clock. No real network and no real time.

## Releasing

Bump the version in `Cargo.toml` and `packages/*/package.json` together, add the CHANGELOG entry, push, then tag `vX.Y.Z`. The Publish crates workflow uploads to crates.io once CI has passed on the tagged commit. A crates.io version cannot be replaced, so push the tag only when the release is final. To retry: `gh workflow run publish.yml -f tag=vX.Y.Z`.
