# BabelPrime

**Verifiable arbitrary-precision integer factorization and primality research engine.**

BabelPrime provides a modular classical factorization system with independently verifiable results, resumable computation, algorithm selection, and interfaces for future distributed and quantum backends.

> **Honest scope statement:** BabelPrime does **not** claim polynomial-time classical integer factorization. General-purpose classical factoring remains subexponential (and in practice exponential for hard instances). Primality testing, by contrast, is in P.

## Features (v0.1.0)

- Arbitrary-precision integers (via `num-bigint`)
- Small-prime trial division
- Perfect-power detection
- GCD and modular arithmetic
- Miller–Rabin probable-prime testing with explicit assurance metadata
- Pollard rho (reproducible seeds)
- Pollard p−1
- Algorithm dispatch with bounded execution budgets
- Recursive factor decomposition
- Versioned, independently verifiable JSON result schema
- Deterministic job configuration and reproducibility metadata
- Independent verification crate

### Explicitly *not* operational in v0.1

- Elliptic Curve Method (ECM)
- Quadratic sieve
- General Number Field Sieve (GNFS)
- Distributed relation collection
- Quantum order-finding backend

Interfaces for these exist as stubs only.

## Architecture ownership

BabelPrime owns:

- Integer factorization
- Primality testing & certificates
- Arbitrary-precision integer interfaces for factoring
- Factorization algorithm selection
- Factorization job state and result schemas
- Factorization-specific benchmarking

It does **not** own general execution authorization, typed state/invariants, tensor infrastructure, independent mathematical validation frameworks, or quantum-gravity research. Integration with other BabelDragon projects occurs only through explicit interfaces; there are no mandatory dependencies on them.

## Quick start

```bash
# Build (Rust 1.70+)
cargo build --release

# Factor an integer
cargo run -p babelprime-api -- factor 143
cargo run -p babelprime-api -- factor 0xC27 --json

# Primality test
cargo run -p babelprime-api -- primality 18446744073709551557

# Verify a result JSON
cargo run -p babelprime-api -- factor 91 --json | cargo run -p babelprime-api -- verify
```

## Result schema (excerpt)

Every result distinguishes proven prime / probable prime / composite, and complete / partial / incomplete factorizations. See [docs/RESULT_SCHEMA.md](docs/RESULT_SCHEMA.md).

## Repository layout

```
crates/
  babelprime-arithmetic/   # GCD, modexp, perfect powers, canonical integers
  babelprime-primality/    # Miller–Rabin + evidence
  babelprime-factor/       # Algorithms + job engine + result types
  babelprime-verify/       # Independent reconstruction & checks
  babelprime-core/         # Public re-exports
  babelprime-api/          # CLI
python/babelprime/         # Research API (thin)
tests/                     # Vectors and integration tests
docs/                      # Architecture, algorithms, security, …
```

## Documentation

- [ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [ALGORITHMS.md](docs/ALGORITHMS.md)
- [RESULT_SCHEMA.md](docs/RESULT_SCHEMA.md)
- [SECURITY.md](docs/SECURITY.md)
- [CONTRIBUTING.md](CONTRIBUTING.md)
- [CHANGELOG.md](CHANGELOG.md)

## License

MIT
