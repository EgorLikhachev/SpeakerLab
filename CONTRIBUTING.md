# Contributing to SpeakerLab

Thanks for your interest in improving SpeakerLab! This document explains
how to set up a development environment and submit changes.

## Development Environment

1. Install [Rust](https://www.rust-lang.org/tools/install) 1.85 or newer.
2. Clone and build:

   ```bash
   git clone https://github.com/EgorLikhachev/SpeakerLab.git
   cd SpeakerLab
   cargo run -p speakerlab
   ```

3. Linux users need GUI packages (see [README](README.md#prerequisites)).

The workspace contains two crates:

- `crates/acoustics` — the physics core. **No UI code allowed here.**
- `crates/app` — the egui GUI. It must only *consume* the core, never
  duplicate formulas.

Please keep this separation: all new physics goes into `acoustics` with
unit tests; all new interface code goes into `app`.

## Branching Model

We follow a simple **GitHub Flow**:

1. Create a branch from `main`.
2. Keep it focused on one topic.
3. Open a pull request back to `main`.

Branch naming convention:

```text
feat/<short-description>      new feature
fix/<short-description>       bug fix
docs/<short-description>      documentation only
refactor/<short-description>  code structure change
test/<short-description>      tests only
```

Examples: `feat/horn-wizard`, `fix/port-end-correction`, `docs/readme-badges`.

## Commit Messages

We use [Conventional Commits](https://www.conventionalcommits.org/).
Format:

```text
<type>(<scope>): <short summary in imperative mood>

[optional body]
```

Common types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `perf`.
Scope is usually the crate or module: `acoustics`, `app`, `ui`, `verify`.

Examples:

```text
feat(acoustics): add passive radiator mass suggestion
fix(app): recompute curves when switching enclosure type
docs(readme): add verification table
test(vented): assert impedance minimum at Fb
```

## Before You Submit

Run all of these locally; CI runs the same checks:

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

For changes to the physics core, also run the independent verification and
make sure it still reports 14/14:

```bash
cargo run -p speakerlab-acoustics --example dump_curves > verify/curves.json
python verify/verify.py
```

If your change intentionally alters reference values (e.g. a recalibrated
model), update `verify/verify.py` expectations **and** explain the reason
in the pull request.

## Pull Request Process

1. Open a PR against `main` and fill in the
   [pull request template](.github/PULL_REQUEST_TEMPLATE.md).
2. Every PR must include:
   - What changed and why.
   - Tests covering new or fixed behavior (unit tests in `acoustics` for
     physics, UI changes at minimum must not break existing tests).
   - Updated documentation if user-visible behavior changed.
3. CI must pass (formatting, clippy, tests on Ubuntu and Windows).
4. A maintainer reviews and merges. Small, focused PRs get faster reviews.

Review checklist (also embedded in the template):

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] Physics changes have unit tests and pass `verify/verify.py`
- [ ] New UI strings are added to **both** `locales/ru.yml` and `locales/en.yml`
- [ ] Documentation updated where needed

## Reporting Issues

Use the issue templates:

- [Bug report](https://github.com/EgorLikhachev/SpeakerLab/issues/new?template=bug_report.md)
- [Feature request](https://github.com/EgorLikhachev/SpeakerLab/issues/new?template=feature_request.md)

For security vulnerabilities, do **not** open a public issue — see
[SECURITY.md](SECURITY.md).

## Code Style

- Formatting is enforced by `rustfmt` (`cargo fmt`).
- Linting is enforced by `clippy` with warnings denied (see commands above).
- Comments in the code base are written in Russian; public documentation
  (README, CONTRIBUTING, CHANGELOG) in English. Either is acceptable in PRs,
  but stay consistent within a file.
- Keep functions small and unit-testable; the physics core must remain UI-free.
