# Architecture

## Goals

- Independently verifiable results
- Resumable, budgeted computation
- Clear separation of algorithms from orchestration
- No silent truncation of large integers
- Honest completeness and primality status

## Crate graph

```
babelprime-api
      │
babelprime-core
      │
      ├── babelprime-factor
      │         ├── babelprime-primality
      │         └── babelprime-arithmetic
      ├── babelprime-verify
      │         ├── babelprime-factor
      │         └── babelprime-primality
      └── (re-exports)
```

`babelprime-arithmetic` is the foundation and has no internal BabelPrime dependencies.

## Job model

A factorization job is described by `FactorJobConfig`:

- deterministic seed
- wall-clock timeout
- operation budget
- Miller–Rabin rounds
- trial-division bound
- feature flags for rho / p−1
- maximum recursion depth

Computation is pure with respect to this configuration. Persistence and distributed orchestration are deliberately outside the core.

## Result ownership

The versioned `FactorizationResult` document is the sole contract between the engine and consumers. Verification is performed by an independent crate that re-checks divisibility, product reconstruction, and primality claims.

## Future backends

Stubs exist for ECM, quadratic sieve, GNFS, and quantum order-finding. They return “not available” and are never selected by the dispatcher.
