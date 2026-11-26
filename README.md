# BabelPrime

**Verifiable arbitrary-precision integer factorization and primality research engine.**

BabelPrime provides a modular classical factorization system with independently verifiable results, algorithm selection under explicit resource budgets, and interfaces for future distributed and quantum backends.

> **Honest scope statement:** BabelPrime does **not** claim polynomial-time classical integer factorization. General-purpose classical factoring remains subexponential (and in practice exponential for hard instances). Primality testing, by contrast, is in P.

## Features (v0.1.0)

- Arbitrary-precision integers (via `num-bigint`)
- Small-prime trial division (including boundary primes such as 251)
- Perfect-power detection
- GCD and modular arithmetic
- Miller–Rabin probable-prime testing with explicit assurance metadata
- Pollard rho (reproducible seeds, bounded retries)
- Pollard p−1 (underflow-safe)
- Algorithm dispatch with wall-clock timeout and operation budgets
- Recursive factor decomposition
- Versioned, independently verifiable JSON result schema
- Deterministic job configuration and full config hashing
- Independent structural verification crate

### Explicitly *not* operational in v0.1

- Elliptic Curve Method (ECM)
- Quadratic sieve
- General Number Field Sieve (GNFS)
- Distributed relation collection
- Quantum order-finding backend
- Checkpoint / resume of mid-job factorization state (planned; not implemented)

Interfaces for future algorithms exist as stubs only.

## What the verifier proves

`babelprime-verify` performs **structural reconstruction only**:

- every listed factor divides the input;
- product of factors (× remaining cofactor) equals the input;
- completeness, verification status, and remaining cofactor are consistent;
- complete results do not list composite-status factors.

It does **not** issue primality certificates. Probable primes remain labelled as such. Use `recheck_primality` for an independent Miller–Rabin re-evaluation.

## Architecture ownership

BabelPrime owns factorization, primality testing, arbitrary-precision interfaces for factoring, algorithm selection, job/result schemas, and factorization-specific benchmarking. It does **not** own general execution authorization, typed state frameworks, tensor infrastructure, or other BabelDragon research domains. There are no mandatory dependencies on other BabelDragon repositories.

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

## Documentation

- [ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [ALGORITHMS.md](docs/ALGORITHMS.md)
- [RESULT_SCHEMA.md](docs/RESULT_SCHEMA.md)
- [SECURITY.md](docs/SECURITY.md)
- [CONTRIBUTING.md](CONTRIBUTING.md)
- [CHANGELOG.md](CHANGELOG.md)

## License

MIT
