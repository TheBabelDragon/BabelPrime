//! Independent verification of BabelPrime factorization results.

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

pub fn verify_result(result: &FactorizationResult) -> Result<(), VerifyError> {
    let input = result.input_integer.to_biguint()?;
    let mut product = BigUint::one();
    for fp in &result.discovered_factors {
        let p = fp.prime.to_biguint()?;
        if p <= BigUint::one() {
            return Err(VerifyError::Failed(format!("invalid factor {}", fp.prime.as_str())));
        }
        if &input % &p != BigUint::zero() {
            return Err(VerifyError::Failed(format!("factor {} does not divide input", fp.prime.as_str())));
        }
        for _ in 0..fp.exponent {
            product *= &p;
        }
    }
    if let Some(ref c) = result.remaining_cofactor {
        product *= c.to_biguint()?;
    }
    if product != input {
        return Err(VerifyError::Failed(format!("product {} != input {}", product.to_str_radix(10), result.input_integer.as_str())));
    }
    match result.completeness {
        Completeness::Complete => {
            if result.remaining_cofactor.is_some() {
                return Err(VerifyError::Failed("complete but has remaining_cofactor".into()));
            }
            if result.verification_status != VerificationStatus::Verified {
                return Err(VerifyError::Failed("complete requires verified status".into()));
            }
            for fp in &result.discovered_factors {
                match &fp.primality {
                    PrimalityStatus::ProvenPrime | PrimalityStatus::ProbablePrime { .. } => {}
                    other => {
                        return Err(VerifyError::Failed(format!("non-prime status {:?} for {}", other, fp.prime.as_str())));
                    }
                }
            }
        }
        Completeness::Partial => {
            if result.remaining_cofactor.is_none() {
                return Err(VerifyError::Failed("partial requires remaining_cofactor".into()));
            }
        }
        Completeness::Incomplete => {}
    }
    Ok(())
}

pub fn recheck_primality(result: &FactorizationResult, rounds: u32, seed: u64) -> Result<bool, VerifyError> {
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
    let result: FactorizationResult =
        serde_json::from_str(json).map_err(|e| VerifyError::Failed(format!("invalid JSON: {e}")))?;
    verify_result(&result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use babelprime_factor::{factorize, FactorJobConfig};

    #[test]
    fn verify_91() {
        let res = factorize(&91u32.into(), &FactorJobConfig::default()).unwrap();
        assert!(verify_result(&res).is_ok());
        assert_eq!(res.completeness, Completeness::Complete);
    }
}
