# Result Schema (v0.1.0)

See the type definitions in `babelprime-factor` for the authoritative schema. Key fields:

- schema_version, algorithm_version
- input_integer (canonical decimal, never truncated)
- discovered_factors: list of {prime, exponent, primality}
- primality_status of the original input
- verification_status: verified | partial | failed
- completeness: complete | partial | incomplete
- execution_metadata (algorithms, bit length, timing, budget flags)
- reproducibility_metadata (seed, config_hash)
- remaining_cofactor (when partial)

Invariants enforced by the verifier:
1. Every listed prime divides the input.
2. Product of p_i^e_i (× remaining cofactor) equals the input.
3. complete implies no remaining cofactor and verification_status = verified.
4. partial implies a remaining cofactor is present.
5. Claimed primes carry proven_prime or probable_prime evidence.
