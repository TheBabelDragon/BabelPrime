# Security

## Design principles

- No network execution by default. The core is a pure computational library.
- No unbounded worker spawning. Recursion depth and operation budgets are explicit.
- No hidden fallback behavior. Unimplemented algorithms are never silently substituted.
- No silent integer truncation. All integers are carried as arbitrary-precision values and serialized in full decimal form.
- No false primality claims. Probable primes are labelled as such with round counts and error-bound estimates.
- No fabricated completion results. Partial and incomplete outcomes are first-class and cannot be misinterpreted as complete factorizations.
- All external string inputs are validated before conversion to integers.

## Threat model (v0.1)

BabelPrime is a research and verification tool, not a cryptographic boundary. It does not protect against:

- maliciously crafted job configurations that exhaust resources (timeouts and budgets mitigate this),
- side-channel leakage from the host process,
- compromise of the host or the language runtime.

Users who treat factorization results as security-critical must independently re-verify them with the supplied verification crate (or an independent implementation of the same checks).

## Dependency policy

Core crates depend only on well-known, widely audited crates (`num-bigint`, `serde`, `rand`/`rand_chacha`, `thiserror`). Optional CLI dependencies are isolated in `babelprime-api`.
