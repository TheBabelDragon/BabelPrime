//! Known-answer regression tests for factorization.

use babelprime_factor::{
    factorize, factorize_str, Completeness, FactorJobConfig, VerificationStatus,
};
use num_bigint::BigUint;
use num_traits::One;

fn cfg_quick() -> FactorJobConfig {
    FactorJobConfig {
        trial_bound: 10_000,
        timeout_ms: 10_000,
        operation_budget: 5_000_000,
        mr_rounds: 12,
        ..Default::default()
    }
}

fn product_matches(res: &babelprime_factor::FactorizationResult, n: &BigUint) -> bool {
    let mut product = BigUint::one();
    for fp in &res.discovered_factors {
        let p = fp.prime.to_biguint().unwrap();
        for _ in 0..fp.exponent {
            product *= &p;
        }
    }
    if let Some(ref c) = res.remaining_cofactor {
        product *= c.to_biguint().unwrap();
    }
    &product == n
}

#[test]
fn factor_2() {
    let n = BigUint::from(2u32);
    let res = factorize(&n, &cfg_quick()).unwrap();
    assert_eq!(res.completeness, Completeness::Complete);
    assert_eq!(res.discovered_factors.len(), 1);
    assert_eq!(res.discovered_factors[0].prime.as_str(), "2");
    assert!(product_matches(&res, &n));
}

#[test]
fn factor_97_prime() {
    let n = BigUint::from(97u32);
    let res = factorize(&n, &cfg_quick()).unwrap();
    assert_eq!(res.completeness, Completeness::Complete);
    assert_eq!(res.discovered_factors[0].prime.as_str(), "97");
    assert!(product_matches(&res, &n));
}

#[test]
fn factor_91() {
    let n = BigUint::from(91u32);
    let res = factorize(&n, &cfg_quick()).unwrap();
    assert_eq!(res.completeness, Completeness::Complete);
    assert_eq!(res.verification_status, VerificationStatus::Verified);
    assert!(product_matches(&res, &n));
    let primes: Vec<_> = res.discovered_factors.iter().map(|f| f.prime.as_str()).collect();
    assert!(primes.contains(&"7"));
    assert!(primes.contains(&"13"));
}

#[test]
fn factor_143() {
    let res = factorize_str("143", Some(cfg_quick())).unwrap();
    assert_eq!(res.completeness, Completeness::Complete);
    assert!(product_matches(&res, &BigUint::from(143u32)));
}

#[test]
fn factor_8051() {
    let n = BigUint::from(8051u32);
    let res = factorize(&n, &cfg_quick()).unwrap();
    assert_eq!(res.completeness, Completeness::Complete);
    assert!(product_matches(&res, &n));
}

#[test]
fn factor_10403() {
    let n = BigUint::from(10403u32);
    let res = factorize(&n, &cfg_quick()).unwrap();
    assert_eq!(res.completeness, Completeness::Complete);
    assert!(product_matches(&res, &n));
}

#[test]
fn factor_360_powers() {
    let n = BigUint::from(360u32);
    let res = factorize(&n, &cfg_quick()).unwrap();
    assert_eq!(res.completeness, Completeness::Complete);
    assert!(product_matches(&res, &n));
}

#[test]
fn factor_perfect_power() {
    let n = BigUint::from(8u32);
    let res = factorize(&n, &cfg_quick()).unwrap();
    assert_eq!(res.completeness, Completeness::Complete);
    assert_eq!(res.discovered_factors.len(), 1);
    assert_eq!(res.discovered_factors[0].exponent, 3);
    assert!(product_matches(&res, &n));
}

#[test]
fn reject_one() {
    assert!(factorize(&BigUint::from(1u32), &cfg_quick()).is_err());
}

#[test]
fn reject_zero() {
    assert!(factorize(&BigUint::from(0u32), &cfg_quick()).is_err());
}

#[test]
fn reject_negative_string() {
    assert!(factorize_str("-15", Some(cfg_quick())).is_err());
}

#[test]
fn reject_malformed() {
    assert!(factorize_str("not-a-number", Some(cfg_quick())).is_err());
}

#[test]
fn budget_exhaustion_partial() {
    let mut cfg = cfg_quick();
    cfg.operation_budget = 5;
    cfg.trial_bound = 3;
    cfg.enable_rho = false;
    cfg.enable_p1 = false;
    let n = BigUint::from(10403u32);
    let res = factorize(&n, &cfg).unwrap();
    assert_ne!(res.completeness, Completeness::Complete);
}

#[test]
fn deterministic_repeatability() {
    let cfg = FactorJobConfig { seed: 12345, ..cfg_quick() };
    let a = factorize(&BigUint::from(8051u32), &cfg).unwrap();
    let b = factorize(&BigUint::from(8051u32), &cfg).unwrap();
    assert_eq!(a.discovered_factors.len(), b.discovered_factors.len());
    for (fa, fb) in a.discovered_factors.iter().zip(b.discovered_factors.iter()) {
        assert_eq!(fa.prime.as_str(), fb.prime.as_str());
        assert_eq!(fa.exponent, fb.exponent);
    }
}
