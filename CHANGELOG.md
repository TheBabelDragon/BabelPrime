# Changelog

## [0.2.0] – 2026-11-26

### Added

- **babelprime-checkpoint**: durable checkpoint/resume with **SHA-256** integrity digests
  (corruption detection only — not a keyed authenticator), collision-resistant temp files,
  atomic replace, and job-identity validation (input, config hash, seed, algorithm, algorithm version).
- **babelprime-ecm**: operational ECM **Stage 1** (Montgomery projective arithmetic, Suyama
  parametrization, deterministic curve seeds, B1 stage-1, budgets, checkpoint payload).
- Factor dispatcher: `enable_ecm`, `ecm_b1`, `ecm_max_curves`; CLI `--ecm`, `--ecm-b1`, `--ecm-curves`.
- Package version **0.2.0**. Result `schema_version` remains **0.1.0** (contract unchanged).
  `algorithm_version` is **0.2.0**.

### Explicit non-goals

- ECM Stage 2, QS, GNFS, distributed collection, quantum backends.
- Checkpoint authenticity (MAC/signature) — integrity only.

## [0.1.0] – 2026-11-26

Initial release: trial division, perfect-power, Miller–Rabin, Pollard rho/p−1, structural verifier, CLI, CI.
