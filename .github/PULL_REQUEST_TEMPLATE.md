## Description

<!-- What does this PR change and why? Link related issues with "Closes #123". -->

## Type of Change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing behavior to change)
- [ ] Documentation only
- [ ] Physics-core change (touches `crates/acoustics`)

## Checklist

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test` passes (32+ tests)
- [ ] Physics changes have unit tests
- [ ] `python verify/verify.py` still passes 14/14 (or expectations updated with justification below)
- [ ] New UI strings added to **both** `crates/app/locales/ru.yml` and `crates/app/locales/en.yml`
- [ ] Documentation updated (README / CHANGELOG) if user-visible behavior changed

## Verification Evidence

<!-- For physics changes: describe how you validated the results
     (tests, verification script, reference values). -->
