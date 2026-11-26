# Result Schema (v0.1.0)

Authoritative types live in `babelprime-factor`.

## Fields

| Field | Meaning |
|-------|---------|
| `schema_version` | Document format version (`0.1.0`) |
| `algorithm_version` | Engine algorithm suite version |
| `input_integer` | Canonical decimal string (no leading zeros, never truncated) |
| `discovered_factors` | `{prime, exponent, primality}` list |
| `primality_status` | Status of the **original** input |
| `verification_status` | `verified` / `partial` / `failed` |
| `completeness` | `complete` / `partial` / `incomplete` |
| `execution_metadata` | algorithms used, bit length, elapsed_ms, ops, budget/timeout flags |
| `reproducibility_metadata` | seed + `config_hash` of the full normalized job config |
| `remaining_cofactor` | Present only when completeness ≠ complete |

`elapsed_ms` is observational and is **not** part of the deterministic identity of a result. Identical configs must produce identical mathematical factors; wall-clock may differ.

## PrimalityStatus

- `proven_prime` — only after complete small-prime trial division up to √n (deterministic bound)
- `probable_prime` `{ rounds, error_bound_bits }` — Miller–Rabin survivors
- `composite`
- `undetermined`

Probable primes are never silently upgraded to proven primes.

## Completeness semantics

- **complete** — product of listed factors equals input; no remaining cofactor; all factors carry proven or probable status; `verification_status = verified`
- **partial** — some factors found; remaining cofactor present; must not claim `verified`
- **incomplete** — no useful factors within budget/timeout

## Verifier invariants (`babelprime-verify`)

1. Canonical decimal forms for input, factors, and cofactor.
2. Every listed factor ≥ 2 divides the input; exponents ≥ 1.
3. Product of p_i^e_i (× remaining cofactor) equals the input.
4. `complete` ⇒ no cofactor, status `verified`, non-empty factors with proven/probable status.
5. `partial` ⇒ cofactor present; must not claim `verified`.
6. `incomplete` ⇒ must not claim `verified`.

**Structural reconstruction is not a primality certificate.**
