//! Primality testing for BabelPrime.
//!
//! Deterministic ProvenPrime is only claimed after complete small-prime trial
//! division up to sqrt(n), or for n == 2. ProbablePrime is never upgraded to
//! ProvenPrime. Zero Miller–Rabin rounds are rejected for candidates that need MR.

use babelprime_arithmetic::{mod_pow, parse_integer, ArithmeticError};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimalityStatus {
    ProvenPrime,
    ProbablePrime { rounds: u32, error_bound_bits: u32 },
    Composite,
    Undetermined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalityEvidence {
    pub status: PrimalityStatus,
    pub algorithm: String,
    pub witnesses: Vec<String>,
    pub deterministic_bound: Option<u64>,
}

#[derive(Debug, Error)]
pub enum PrimalityError {
    #[error(transparent)]
    Arithmetic(#[from] ArithmeticError),
    #[error("miller-rabin rounds must be >= 1 for non-trivial candidates")]
    ZeroRounds,
}

/// Primes used for trial screening. Includes 251 so that 251² = 63001 is
/// correctly identified as composite during screening.
const SMALL_PRIMES: &[u64] = &[
    2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61, 67, 71, 73, 79, 83, 89,
    97, 101, 103, 107, 109, 113, 127, 131, 137, 139, 149, 151, 157, 163, 167, 173, 179, 181,
    191, 193, 197, 199, 211, 223, 227, 229, 233, 239, 241, 251,
];

/// Largest prime in SMALL_PRIMES. ProvenPrime via trial is only claimed when
/// sqrt(n) <= this bound (i.e. n <= LAST_SMALL_PRIME²).
const LAST_SMALL_PRIME: u64 = 251;

pub fn miller_rabin(
    n: &BigUint,
    rounds: u32,
    seed: u64,
) -> Result<PrimalityEvidence, PrimalityError> {
    if n.is_zero() || n.is_one() {
        return Ok(ev(PrimalityStatus::Composite, "trivial", vec![], None));
    }
    if n == &BigUint::from(2u32) {
        return Ok(ev(PrimalityStatus::ProvenPrime, "trivial", vec![], Some(2)));
    }
    if !n.bit(0) {
        return Ok(ev(
            PrimalityStatus::Composite,
            "trial_division",
            vec!["2".into()],
            None,
        ));
    }

    let sqrt_n = n.sqrt();
    let last = BigUint::from(LAST_SMALL_PRIME);

    for &p in SMALL_PRIMES.iter().skip(1) {
        let pb = BigUint::from(p);
        // Fully screened: every prime <= sqrt(n) was tested.
        if pb > sqrt_n {
            return Ok(ev(PrimalityStatus::ProvenPrime, "trial_division", vec![], Some(p)));
        }
        if n % &pb == BigUint::zero() {
            return Ok(ev(
                PrimalityStatus::Composite,
                "trial_division",
                vec![p.to_string()],
                None,
            ));
        }
    }

    // If sqrt(n) <= LAST_SMALL_PRIME, screening was complete → proven prime.
    if sqrt_n <= last {
        return Ok(ev(
            PrimalityStatus::ProvenPrime,
            "trial_division",
            vec![],
            Some(LAST_SMALL_PRIME),
        ));
    }

    // Beyond deterministic screening bound: Miller–Rabin required.
    if rounds == 0 {
        return Err(PrimalityError::ZeroRounds);
    }

    let (d, s) = write_odd(n);
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    let mut witnesses = Vec::new();

    for _ in 0..rounds {
        let a = random_base(&mut rng, n);
        if !strong_liar(n, &a, &d, s)? {
            return Ok(ev(
                PrimalityStatus::Composite,
                "miller_rabin",
                vec![a.to_str_radix(10)],
                None,
            ));
        }
        witnesses.push(a.to_str_radix(10));
    }

    // Conservative error bound: ~4^{-rounds} under random bases.
    Ok(ev(
        PrimalityStatus::ProbablePrime {
            rounds,
            error_bound_bits: rounds.saturating_mul(2),
        },
        "miller_rabin",
        witnesses,
        None,
    ))
}

fn ev(status: PrimalityStatus, algo: &str, w: Vec<String>, bound: Option<u64>) -> PrimalityEvidence {
    PrimalityEvidence {
        status,
        algorithm: algo.into(),
        witnesses: w,
        deterministic_bound: bound,
    }
}

fn write_odd(n: &BigUint) -> (BigUint, u32) {
    let mut d = n - BigUint::one();
    let mut s = 0u32;
    while !d.bit(0) {
        d >>= 1u32;
        s += 1;
    }
    (d, s)
}

fn random_base<R: Rng>(rng: &mut R, n: &BigUint) -> BigUint {
    // Uniform in [2, n-2]
    let range = n - BigUint::from(3u32);
    loop {
        let mut bytes = vec![0u8; (range.bits() as usize / 8) + 2];
        rng.fill_bytes(&mut bytes);
        let c = BigUint::from_bytes_be(&bytes) % &range + BigUint::from(2u32);
        if c < n.clone() - BigUint::one() {
            return c;
        }
    }
}

fn strong_liar(n: &BigUint, a: &BigUint, d: &BigUint, s: u32) -> Result<bool, PrimalityError> {
    let mut x = mod_pow(a, d, n)?;
    if x.is_one() || x == n - BigUint::one() {
        return Ok(true);
    }
    for _ in 1..s {
        x = mod_pow(&x, &BigUint::from(2u32), n)?;
        if x == n - BigUint::one() {
            return Ok(true);
        }
        if x.is_one() {
            return Ok(false);
        }
    }
    Ok(false)
}

pub fn is_probable_prime(n: &BigUint, rounds: u32, seed: u64) -> Result<bool, PrimalityError> {
    let e = miller_rabin(n, rounds, seed)?;
    Ok(matches!(
        e.status,
        PrimalityStatus::ProvenPrime | PrimalityStatus::ProbablePrime { .. }
    ))
}

pub fn test_primality_str(
    s: &str,
    rounds: u32,
    seed: u64,
) -> Result<PrimalityEvidence, PrimalityError> {
    miller_rabin(&parse_integer(s)?, rounds, seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_primes_proven() {
        for p in [2u32, 3, 5, 7, 11, 13, 17, 19, 23, 251] {
            let e = miller_rabin(&p.into(), 8, 1).unwrap();
            assert!(matches!(e.status, PrimalityStatus::ProvenPrime), "{p}");
        }
    }

    #[test]
    fn composite_251_squared() {
        // 63001 = 251² — must be Composite, never ProvenPrime
        let n = BigUint::from(63001u32);
        let e = miller_rabin(&n, 16, 42).unwrap();
        assert!(matches!(e.status, PrimalityStatus::Composite), "{e:?}");
    }

    #[test]
    fn composites() {
        for c in [4u32, 9, 15, 21, 25, 27, 33, 35, 561, 63001] {
            let e = miller_rabin(&c.into(), 16, 42).unwrap();
            assert!(!matches!(e.status, PrimalityStatus::ProvenPrime), "{c}");
        }
    }

    #[test]
    fn zero_rounds_rejected_for_large() {
        // 63521 is prime and floor(sqrt(63521)) = 252 > 251, so screening is incomplete.
        let n = BigUint::from(63521u32);
        let err = miller_rabin(&n, 0, 1);
        assert!(matches!(err, Err(PrimalityError::ZeroRounds)), "{err:?}");
    }

    #[test]
    fn zero_rounds_ok_for_small_proven() {
        // Fully screened primes still work with rounds=0
        let e = miller_rabin(&97u32.into(), 0, 1).unwrap();
        assert!(matches!(e.status, PrimalityStatus::ProvenPrime));
    }
}
