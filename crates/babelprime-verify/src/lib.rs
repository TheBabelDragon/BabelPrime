//! Independent verification of BabelPrime factorization results.
//!
//! # What the verifier proves
//!
//! Structural reconstruction only:
//! - every listed factor is an integer ≥ 2 that divides the input;
//! - the product of p_i^e_i (× remaining cofactor if present) equals the input;
//! - completeness / verification_status / remaining_cofactor are mutually consistent;
//! - complete results only list factors with proven_prime or probable_prime status
//!   (the verifier does **not** upgrade or certify primality — it only checks that
//!   the engine did not claim composite factors in a complete factorization).
//!
//! Structural reconstruction is **not** a primality certificate. Use
//! `recheck_primality` for an independent Miller–Rabin re-evaluation.

use babelprime_arithmetic::ArithmeticError;
use babelprime_factor::{Completeness, FactorizationResult, VerificationStatus};
use babelprime_primality::{miller_rabin, PrimalityStatus};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VerifyError {
    #[error(transparent)]
    Arithmetic(#[from] ArithmeticError),
    #[error("verification failed: {0}")]
    Failed(String),
}

/// Independently verify structural consistency of a factorization result.
pub fn verify_result(result: &FactorizationResult) -> Result<(), VerifyError> {
    // Canonical decimal: no leading zeros (except "0" which is invalid as input anyway)
    let input_s = result.input_integer.as_str();
    if input_s.is_empty()
        || (input_s.len() > 1 && input_s.starts_with('0'))
        || input_s.starts_with('-')
        || !input_s.bytes().all(|b| b.is_ascii_digit())
    {
        return Err(VerifyError::Failed(format!("non-canonical input_integer: {input_s}")));
    }
    let input = result.input_integer.to_biguint()?;

    let mut product = BigUint::one();
    for fp in &result.discovered_factors {
        let ps = fp.prime.as_str();
        if ps.is_empty()
            || (ps.len() > 1 && ps.starts_with('0'))
            || !ps.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(VerifyError::Failed(format!("non-canonical factor: {ps}")));
        }
        if fp.exponent == 0 {
            return Err(VerifyError::Failed(format!("zero exponent for {ps}")));
        }
        let p = fp.prime.to_biguint()?;
        if p <= BigUint::one() {
            return Err(VerifyError::Failed(format!("invalid factor {ps}")));
        }
        if &input % &p != BigUint::zero() {
            return Err(VerifyError::Failed(format!("factor {ps} does not divide input")));
        }
        for _ in 0..fp.exponent {
            product *= &p;
        }
    }

    if let Some(ref c) = result.remaining_cofactor {
        let cs = c.as_str();
        if cs.is_empty()
            || (cs.len() > 1 && cs.starts_with('0'))
            || !cs.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(VerifyError::Failed(format!("non-canonical remaining_cofactor: {cs}")));
        }
        let cv = c.to_biguint()?;
        if cv <= BigUint::one() {
            return Err(VerifyError::Failed("remaining_cofactor must be >= 2".into()));
        }
        product *= cv;
    }

    if product != input {
        return Err(VerifyError::Failed(format!(
            "product {} != input {}",
            product.to_str_radix(10),
            input_s
        )));
    }

    match result.completeness {
        Completeness::Complete => {
            if result.remaining_cofactor.is_some() {
                return Err(VerifyError::Failed("complete but has remaining_cofactor".into()));
            }
            if result.verification_status != VerificationStatus::Verified {
                return Err(VerifyError::Failed("complete requires verification_status=verified".into()));
            }
            if result.discovered_factors.is_empty() {
                return Err(VerifyError::Failed("complete factorization has no factors".into()));
            }
            for fp in &result.discovered_factors {
                match &fp.primality {
                    PrimalityStatus::ProvenPrime | PrimalityStatus::ProbablePrime { .. } => {}
                    other => {
                        return Err(VerifyError::Failed(format!(
                            "complete lists non-prime status {other:?} for {}",
                            fp.prime.as_str()
                        )));
                    }
                }
            }
            // Probable primes are allowed in complete results; they must not be
            // silently treated as proven. The status field itself carries the distinction.
        }
        Completeness::Partial => {
            if result.remaining_cofactor.is_none() {
                return Err(VerifyError::Failed("partial requires remaining_cofactor".into()));
            }
            if result.verification_status == VerificationStatus::Verified {
                return Err(VerifyError::Failed(
                    "partial must not claim verification_status=verified".into(),
                ));
            }
        }
        Completeness::Incomplete => {
            if result.verification_status == VerificationStatus::Verified {
                return Err(VerifyError::Failed("incomplete must not claim verified".into()));
            }
        }
    }

    Ok(())
}

/// Independently re-run Miller–Rabin on every claimed factor.
/// Returns Ok(true) if all factors pass as proven or probable primes.
pub fn recheck_primality(
    result: &FactorizationResult,
    rounds: u32,
    seed: u64,
) -> Result<bool, VerifyError> {
    if rounds == 0 {
        return Err(VerifyError::Failed("rounds must be >= 1".into()));
    }
    for fp in &result.discovered_factors {
        let p = fp.prime.to_biguint()?;
        let ev = miller_rabin(&p, rounds, seed).map_err(|e| VerifyError::Failed(e.to_string()))?;
        if matches!(ev.status, PrimalityStatus::Composite | PrimalityStatus::Undetermined) {
            return Ok(false);
        }
    }
    Ok(true)
}

pub fn verify_json(json: &str) -> Result<(), VerifyError> {
    let result: FactorizationResult = serde_json::from_str(json)
        .map_err(|e| VerifyError::Failed(format!("invalid JSON: {e}")))?;
    verify_result(&result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use babelprime_arithmetic::CanonicalInteger;
    use babelprime_factor::{factorize, FactorJobConfig, FactorPower};

    #[test]
    fn verify_91() {
        let res = factorize(&91u32.into(), &FactorJobConfig::default()).unwrap();
        assert!(verify_result(&res).is_ok());
        assert_eq!(res.completeness, Completeness::Complete);
    }

    #[test]
    fn reject_tampered_factor() {
        let mut res = factorize(&15u32.into(), &FactorJobConfig::default()).unwrap();
        res.discovered_factors[0].prime = CanonicalInteger("4".into());
        assert!(verify_result(&res).is_err());
    }

    #[test]
    fn reject_complete_with_cofactor() {
        let mut res = factorize(&15u32.into(), &FactorJobConfig::default()).unwrap();
        res.remaining_cofactor = Some(CanonicalInteger("2".into()));
        res.completeness = Completeness::Complete;
        assert!(verify_result(&res).is_err());
    }

    #[test]
    fn reject_partial_without_cofactor() {
        let mut res = factorize(&15u32.into(), &FactorJobConfig::default()).unwrap();
        res.completeness = Completeness::Partial;
        res.verification_status = VerificationStatus::Partial;
        res.remaining_cofactor = None;
        assert!(verify_result(&res).is_err());
    }

    #[test]
    fn reject_zero_exponent() {
        let mut res = factorize(&15u32.into(), &FactorJobConfig::default()).unwrap();
        res.discovered_factors.push(FactorPower {
            prime: CanonicalInteger("3".into()),
            exponent: 0,
            primality: PrimalityStatus::ProvenPrime,
        });
        assert!(verify_result(&res).is_err());
    }
}
