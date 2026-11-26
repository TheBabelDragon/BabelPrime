# BabelPrime v0.1.0 Release Notes

**Validated commit:** `67adb03d60922c46e6525741cce04d9b2b6e624d`

## Implemented algorithms

- Small-prime trial division (screening through **251**)
- Perfect-power detection
- Miller–Rabin probable-prime testing with explicit assurance metadata (`ProvenPrime` / `ProbablePrime` / `Composite`)
- Pollard rho (seeded, bounded retries)
- Pollard p−1 (underflow-safe)
- Recursive factor decomposition under explicit wall-clock and operation budgets

## Verification

Independent structural verification (`babelprime-verify`):

- Reconstructs the product of reported factors (× remaining cofactor)
- Checks completeness / status / cofactor consistency
- Rejects non-canonical integers, zero exponents, and tampered results
- **Does not** upgrade probable primes to proven primes or issue primality certificates

## Interfaces

- Rust workspace crates: `babelprime-arithmetic`, `babelprime-primality`, `babelprime-factor`, `babelprime-verify`, `babelprime-core`, `babelprime-api`
- CLI: `babelprime factor | primality | verify | info`
- Thin Python package (subprocess CLI wrapper)
- Versioned JSON result schema (`schema_version` 0.1.0)

## Explicitly not operational

- ECM, quadratic sieve, GNFS
- Distributed relation collection
- Quantum order-finding
- Checkpoint / resume of mid-job state

BabelPrime does **not** claim polynomial-time classical general-purpose integer factorization.

## Validation snapshot

- `cargo fmt --check`, `clippy -D warnings`, `cargo test --workspace`, release build: pass
- Python integration tests against real CLI: pass
- CI matrix (Linux / macOS / Windows + Python): success on `67adb03`
- CLI examples: factor 143, factor `0xC27`, primality of `18446744073709551557`, verify pipe: pass

## License

MIT
