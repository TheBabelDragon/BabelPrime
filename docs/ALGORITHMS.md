# Algorithms (v0.1)

## Operational

### Trial division
Deterministic search for factors up to a configurable bound. Used both for small complete factorizations and as a first stage for larger inputs.

### Perfect-power detection
Tests whether n = a^b for b ≥ 2 by searching integer roots from the largest plausible exponent downward. When found, the base is factored and exponents are multiplied.

### GCD & modular arithmetic
Standard binary GCD, extended GCD, modular inverse, and modular exponentiation. All operations are exact.

### Miller–Rabin
Probable-prime test with explicit rounds and a seed-controlled base generator (ChaCha20). Small inputs fall back to deterministic trial division and are reported as `ProvenPrime`. Larger inputs that survive the configured rounds are reported as `ProbablePrime` together with an error-bound estimate in bits. A composite witness yields `Composite`.

### Pollard rho
Classic Pollard rho with a quadratic iterator and GCD steps. The constant and starting point are derived from a reproducible seed. Bounded by the job operation budget and timeout.

### Pollard p−1
Stage-1 only. Accumulates a^k! (approximately) up to a smoothness bound derived from the trial bound. Returns a non-trivial factor when the smoothness condition is met.

### Recursive decomposition
After each non-trivial factor is found it is divided out completely (all powers). The cofactor and, when necessary, the factor itself are re-submitted to the dispatcher.

## Not operational (stubs only)

- Elliptic Curve Method (ECM)
- Quadratic sieve
- General Number Field Sieve
- Distributed relation collection
- Quantum order-finding

These will never be reported as used in `algorithms_used` until implemented and tested.

## Complexity honesty

- Primality testing is in P (AKS) and practical with Miller–Rabin + deterministic witnesses for moderate sizes.
- General-purpose classical integer factorization has no known polynomial-time algorithm. The best general methods are subexponential (GNFS). BabelPrime’s v0.1 algorithms are exponential in the bit length of the smallest prime factor for the hard case.
