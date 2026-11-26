# Changelog

## [0.1.0] – 2026-11-26

### Added

- Initial public release of BabelPrime.
- Crates: arithmetic, primality, factor, verify, core, api.
- Algorithms: trial division, perfect-power detection, Miller–Rabin, Pollard rho, Pollard p−1.
- Versioned JSON result schema with completeness and verification status.
- Independent structural verification of factorizations.
- Deterministic job configuration and platform-stable config hashing (FNV-1a).
- CLI (`babelprime`).
- Documentation: architecture, algorithms, result schema, security.
- CI for Linux / macOS / Windows.
- Known-answer regression suite (including 251, 251², budget exhaustion, determinism).

### Correctness hardening

- Small-prime table extended through 251; deterministic ProvenPrime only within screening bound.
- Zero Miller–Rabin rounds rejected for candidates beyond the screening bound.
- Budget and timeout checks inside trial, rho, p−1, divide-out, and power-of-two loops.
- Pollard p−1 guards against BigUint underflow on a−1.
- Verifier rejects non-canonical decimals, zero exponents, and inconsistent completeness/status/cofactor combinations.
- Verifier semantics documented: structural reconstruction ≠ primality certificate.

### Explicit non-goals for 0.1.0

- ECM, quadratic sieve, GNFS, distributed collection, quantum backends.
- Checkpoint / resume of mid-job state.
