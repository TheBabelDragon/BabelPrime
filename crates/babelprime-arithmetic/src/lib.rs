//! Arbitrary-precision integer arithmetic for BabelPrime.

use num_bigint::{BigInt, BigUint, Sign};
use num_integer::Integer;
use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ArithmeticError {
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("modulus must be positive")]
    NonPositiveModulus,
}

/// Canonical decimal serialization — never truncates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CanonicalInteger(pub String);

impl CanonicalInteger {
    pub fn from_biguint(n: &BigUint) -> Self {
        Self(n.to_str_radix(10))
    }
    pub fn to_biguint(&self) -> Result<BigUint, ArithmeticError> {
        BigUint::parse_bytes(self.0.as_bytes(), 10)
            .ok_or_else(|| ArithmeticError::InvalidInput(format!("invalid integer: {}", self.0)))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn gcd(a: &BigUint, b: &BigUint) -> BigUint {
    a.gcd(b)
}

pub fn extended_gcd(a: &BigInt, b: &BigInt) -> (BigInt, BigInt, BigInt) {
    if b.is_zero() {
        return (a.clone(), BigInt::one(), BigInt::zero());
    }
    let (g, x1, y1) = extended_gcd(b, &(a % b));
    (g, y1.clone(), x1 - (a / b) * y1)
}

pub fn mod_inverse(a: &BigUint, m: &BigUint) -> Option<BigUint> {
    if m <= &BigUint::one() {
        return None;
    }
    let ai = BigInt::from_biguint(Sign::Plus, a.clone());
    let mi = BigInt::from_biguint(Sign::Plus, m.clone());
    let (g, x, _) = extended_gcd(&ai, &mi);
    if g != BigInt::one() {
        return None;
    }
    (((x % &mi) + &mi) % &mi).to_biguint()
}

pub fn mod_pow(
    base: &BigUint,
    exp: &BigUint,
    modulus: &BigUint,
) -> Result<BigUint, ArithmeticError> {
    if modulus.is_zero() {
        return Err(ArithmeticError::NonPositiveModulus);
    }
    if modulus.is_one() {
        return Ok(BigUint::zero());
    }
    Ok(base.modpow(exp, modulus))
}

pub fn isqrt(n: &BigUint) -> BigUint {
    n.sqrt()
}

pub fn bit_length(n: &BigUint) -> u64 {
    n.bits()
}

/// Parse decimal or 0x-hex string to BigUint. Rejects negatives and empty.
pub fn parse_integer(s: &str) -> Result<BigUint, ArithmeticError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ArithmeticError::InvalidInput("empty".into()));
    }
    if s.starts_with('-') {
        return Err(ArithmeticError::InvalidInput("negative not supported".into()));
    }
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        BigUint::parse_bytes(hex.as_bytes(), 16)
            .ok_or_else(|| ArithmeticError::InvalidInput(format!("bad hex: {s}")))
    } else {
        BigUint::parse_bytes(s.as_bytes(), 10)
            .ok_or_else(|| ArithmeticError::InvalidInput(format!("bad decimal: {s}")))
    }
}

/// Detect perfect power n = base^exp (exp >= 2). Prefers largest exponent.
pub fn perfect_power(n: &BigUint) -> Option<(BigUint, u32)> {
    if n <= &BigUint::one() {
        return None;
    }
    let max_exp = n.bits().max(2) as u32;
    for exp in (2..=max_exp).rev() {
        if let Some(base) = integer_nth_root(n, exp) {
            if pow_u32(&base, exp) == *n {
                return Some((base, exp));
            }
        }
    }
    None
}

fn integer_nth_root(n: &BigUint, e: u32) -> Option<BigUint> {
    if e == 0 {
        return None;
    }
    if e == 1 {
        return Some(n.clone());
    }
    let mut lo = BigUint::one();
    let mut hi = n.clone();
    while lo < hi {
        let mid = (&lo + &hi + BigUint::one()) >> 1u32;
        if pow_u32(&mid, e) <= *n {
            lo = mid;
        } else {
            hi = mid - BigUint::one();
        }
    }
    Some(lo)
}

fn pow_u32(base: &BigUint, mut exp: u32) -> BigUint {
    let mut r = BigUint::one();
    let mut b = base.clone();
    while exp > 0 {
        if exp & 1 == 1 {
            r *= &b;
        }
        b = &b * &b;
        exp >>= 1;
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcd_basic() {
        assert_eq!(gcd(&48u32.into(), &18u32.into()), 6u32.into());
    }

    #[test]
    fn perfect_power_8() {
        assert_eq!(perfect_power(&8u32.into()), Some((2u32.into(), 3)));
    }

    #[test]
    fn parse_hex() {
        assert_eq!(parse_integer("0x10").unwrap(), 16u32.into());
    }

    #[test]
    fn reject_negative() {
        assert!(parse_integer("-3").is_err());
    }
}
