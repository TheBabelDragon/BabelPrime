# Changelog

## [0.1.0] – 2026-11-26

### Added

- Initial public release of BabelPrime.
- Crates: arithmetic, primality, factor, verify, core, api.
- Algorithms: trial division, perfect-power detection, Miller–Rabin, Pollard rho, Pollard p−1.
- Versioned JSON result schema with completeness and verification status.
- Independent verification of factorizations.
- Deterministic job configuration and reproducibility metadata.
- CLI (`babelprime`).
- Documentation: architecture, algorithms, result schema, security.
- CI skeleton for Linux/macOS/Windows.

### Explicit non-goals for 0.1.0

- ECM, quadratic sieve, GNFS, distributed collection, quantum backends.
