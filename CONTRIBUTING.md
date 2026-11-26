# Contributing to BabelPrime

Thank you for interest in improving BabelPrime.

## Principles

1. Correctness and verifiability over speed.
2. Honest status reporting (never claim a result is complete or a number is proven prime without evidence).
3. Keep the core free of mandatory dependencies on other BabelDragon repositories.
4. Prefer small, reviewable changes with tests.

## Development workflow

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

Add deterministic test vectors under `tests/vectors/`. Prefer known factorizations over random hard instances for regression tests.

## Pull requests

- Target the `main` branch (or the active release branch).
- Include a clear description of the change and any limitations.
- Update CHANGELOG.md for user-visible changes.
- Do not mark unimplemented algorithms as operational.

## Code of conduct

Be respectful. This is a research codebase; technical disagreement is welcome, personal attacks are not.
