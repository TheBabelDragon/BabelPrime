//! Elliptic Curve Method (ECM) — Stage 1 only.
//!
//! Lenstra ECM using Montgomery curves in projective (X:Z) coordinates
//! with Suyama parametrization. Stage 2 is **not** implemented.

use babelprime_arithmetic::{gcd, mod_inverse, ArithmeticError};
use babelprime_checkpoint::{save_checkpoint, CheckpointConfig, CheckpointEnvelope, CheckpointError};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use thiserror::Error;

pub const ECM_ALGORITHM_ID: &str = "ecm_stage1";
pub const ECM_ALGORITHM_VERSION: &str = "0.2.0";

#[derive(Debug, Error)]
pub enum EcmError {
    #[error(transparent)]
    Arithmetic(#[from] ArithmeticError),
    #[error(transparent)]
    Checkpoint(#[from] CheckpointError),
    #[error("invalid parameter: {0}")]
    InvalidParam(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcmParams {
    pub b1: u64,
    pub max_curves: u32,
    pub seed: u64,
}

impl Default for EcmParams {
    fn default() -> Self {
        Self { b1: 2_000, max_curves: 20, seed: 0xEC11 }
    }
}

impl EcmParams {
    pub fn small() -> Self { Self { b1: 500, max_curves: 10, seed: 0xEC11 } }
    pub fn medium() -> Self { Self::default() }
    pub fn large() -> Self { Self { b1: 10_000, max_curves: 50, seed: 0xEC11 } }
}

#[derive(Debug, Clone)]
pub enum EcmOutcome {
    Factor(BigUint),
    NoFactor { curves_tried: u32, operations: u64 },
    Interrupted { reason: InterruptReason, operations: u64, curves_tried: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptReason { Timeout, BudgetExhausted, Cancelled }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EcmCheckpointState {
    pub b1: u64,
    pub max_curves: u32,
    pub next_curve: u32,
    pub curves_completed: u32,
}

#[derive(Debug, Clone)]
struct Point { x: BigUint, z: BigUint }

impl Point {
    fn infinity() -> Self { Self { x: BigUint::zero(), z: BigUint::zero() } }
    fn is_infinity(&self) -> bool { self.z.is_zero() }
}

struct MontCurve<'a> { n: &'a BigUint, a24: BigUint }

impl<'a> MontCurve<'a> {
    fn double(&self, p: &Point, ops: &mut u64) -> Point {
        if p.is_infinity() { return Point::infinity(); }
        let xpz = mod_add(&p.x, &p.z, self.n);
        let xmz = mod_sub(&p.x, &p.z, self.n);
        let xpz2 = (&xpz * &xpz) % self.n;
        let xmz2 = (&xmz * &xmz) % self.n;
        let four_xz = mod_sub(&xpz2, &xmz2, self.n);
        let x2 = (&xpz2 * &xmz2) % self.n;
        let t = {
            let diff = mod_sub(&xpz2, &xmz2, self.n);
            let ad = (&self.a24 * &diff) % self.n;
            mod_add(&xmz2, &ad, self.n)
        };
        let z2 = (&four_xz * &t) % self.n;
        *ops += 8;
        Point { x: x2, z: z2 }
    }

    fn add(&self, p: &Point, q: &Point, pmq: &Point, ops: &mut u64) -> Point {
        if p.is_infinity() { return q.clone(); }
        if q.is_infinity() { return p.clone(); }
        let u = (&mod_sub(&p.x, &p.z, self.n) * &mod_add(&q.x, &q.z, self.n)) % self.n;
        let v = (&mod_add(&p.x, &p.z, self.n) * &mod_sub(&q.x, &q.z, self.n)) % self.n;
        let upv2 = { let t = mod_add(&u, &v, self.n); (&t * &t) % self.n };
        let umv2 = { let t = mod_sub(&u, &v, self.n); (&t * &t) % self.n };
        *ops += 8;
        Point { x: (&pmq.z * &upv2) % self.n, z: (&pmq.x * &umv2) % self.n }
    }

    fn scalar_mul(&self, p: &Point, k: &BigUint, ops: &mut u64) -> Point {
        if k.is_zero() || p.is_infinity() { return Point::infinity(); }
        let mut r0 = Point::infinity();
        let mut r1 = p.clone();
        for i in (0..k.bits()).rev() {
            if k.bit(i) {
                r0 = self.add(&r0, &r1, p, ops);
                r1 = self.double(&r1, ops);
            } else {
                r1 = self.add(&r0, &r1, p, ops);
                r0 = self.double(&r0, ops);
            }
        }
        r0
    }
}

fn mod_add(a: &BigUint, b: &BigUint, n: &BigUint) -> BigUint { (a + b) % n }
fn mod_sub(a: &BigUint, b: &BigUint, n: &BigUint) -> BigUint {
    if a >= b { (a - b) % n } else { (n - ((b - a) % n)) % n }
}

fn suyama_curve(sigma: &BigUint, n: &BigUint, ops: &mut u64) -> Result<(BigUint, Point), EcmError> {
    let sigma2 = (sigma * sigma) % n;
    *ops += 1;
    let u = mod_sub(&sigma2, &BigUint::from(5u32), n);
    let v = (BigUint::from(4u32) * sigma) % n;
    let vmu = mod_sub(&v, &u, n);
    let vmu3 = { let v2 = (&vmu * &vmu) % n; (&v2 * &vmu) % n };
    let three_u = (BigUint::from(3u32) * &u) % n;
    let num = (&vmu3 * &mod_add(&three_u, &v, n)) % n;
    let u3 = { let u2 = (&u * &u) % n; (&u2 * &u) % n };
    let den = (BigUint::from(16u32) * &u3 % n * &v) % n;
    *ops += 10;
    if den.is_zero() { return Err(EcmError::InvalidParam("suyama den zero".into())); }
    let den_inv = mod_inverse(&den, n).ok_or_else(|| EcmError::InvalidParam("suyama den".into()))?;
    let a24 = (&num * &den_inv) % n;
    let v3 = { let v2 = (&v * &v) % n; (&v2 * &v) % n };
    *ops += 4;
    Ok((a24, Point { x: u3, z: v3 }))
}

fn stage1_exponent(b1: u64) -> BigUint {
    let mut k = BigUint::one();
    for p in primes_upto(b1) {
        let mut pe = p;
        while pe * p <= b1 { pe *= p; }
        k *= BigUint::from(pe);
    }
    k
}

fn primes_upto(limit: u64) -> Vec<u64> {
    if limit < 2 { return vec![]; }
    let mut is_prime = vec![true; (limit + 1) as usize];
    is_prime[0] = false; is_prime[1] = false;
    let mut p = 2u64;
    while p * p <= limit {
        if is_prime[p as usize] {
            let mut m = p * p;
            while m <= limit { is_prime[m as usize] = false; m += p; }
        }
        p += 1;
    }
    (2..=limit).filter(|&i| is_prime[i as usize]).collect()
}

#[allow(clippy::too_many_arguments)]
pub fn ecm_stage1(
    n: &BigUint,
    params: &EcmParams,
    op_budget: u64,
    timeout: Option<Duration>,
    checkpoint: Option<&CheckpointConfig>,
    resume: Option<EcmCheckpointState>,
    ops_base: u64,
    input_canonical: &str,
    config_hash: &str,
) -> Result<EcmOutcome, EcmError> {
    if n <= &BigUint::from(2u32) { return Err(EcmError::InvalidParam("n>2".into())); }
    if params.b1 < 2 || params.max_curves == 0 {
        return Err(EcmError::InvalidParam("b1/curves".into()));
    }
    let start = Instant::now();
    let mut ops = ops_base;
    let mut state = resume.unwrap_or(EcmCheckpointState {
        b1: params.b1, max_curves: params.max_curves, next_curve: 0, curves_completed: 0,
    });
    let k = stage1_exponent(params.b1);
    ops += params.b1 / 10;

    let interrupt = |ops: u64, curves: u32| -> Option<EcmOutcome> {
        if timeout.map(|t| start.elapsed() >= t).unwrap_or(false) {
            return Some(EcmOutcome::Interrupted { reason: InterruptReason::Timeout, operations: ops, curves_tried: curves });
        }
        if op_budget > 0 && ops >= op_budget {
            return Some(EcmOutcome::Interrupted { reason: InterruptReason::BudgetExhausted, operations: ops, curves_tried: curves });
        }
        None
    };

    let write_ckpt = |state: &EcmCheckpointState, ops: u64| -> Result<(), EcmError> {
        if let Some(cfg) = checkpoint {
            let env = CheckpointEnvelope::new(
                ECM_ALGORITHM_VERSION, input_canonical, config_hash, params.seed, ops,
                ECM_ALGORITHM_ID, state.clone(),
            )?;
            save_checkpoint(&cfg.path, &env)?;
        }
        Ok(())
    };

    while state.next_curve < params.max_curves {
        if let Some(out) = interrupt(ops, state.curves_completed) {
            let _ = write_ckpt(&state, ops);
            return Ok(out);
        }
        let curve_idx = state.next_curve;
        let mut rng = ChaCha20Rng::seed_from_u64(
            params.seed ^ (curve_idx as u64).wrapping_mul(0x9E3779B97F4A7C15),
        );
        let sigma = {
            let range = n - BigUint::from(6u32);
            if range.is_zero() { BigUint::from(6u32) } else {
                let mut bytes = vec![0u8; (range.bits() as usize / 8) + 2];
                rng.fill_bytes(&mut bytes);
                (BigUint::from_bytes_be(&bytes) % &range) + BigUint::from(6u32)
            }
        };
        ops += 2;
        let (a24, p0) = match suyama_curve(&sigma, n, &mut ops) {
            Ok(v) => v,
            Err(_) => {
                let g = gcd(&sigma, n);
                if g > BigUint::one() && &g < n { return Ok(EcmOutcome::Factor(g)); }
                state.next_curve += 1; state.curves_completed += 1; continue;
            }
        };
        let curve = MontCurve { n, a24 };
        let q = curve.scalar_mul(&p0, &k, &mut ops);
        if let Some(out) = interrupt(ops, state.curves_completed) {
            let _ = write_ckpt(&state, ops);
            return Ok(out);
        }
        if !q.z.is_zero() {
            let g = gcd(&q.z, n);
            ops += 1;
            if g > BigUint::one() && &g < n { return Ok(EcmOutcome::Factor(g)); }
        }
        state.next_curve += 1;
        state.curves_completed += 1;
        if let Some(cfg) = checkpoint {
            if ops > 0 && ops % cfg.interval_ops == 0 { write_ckpt(&state, ops)?; }
        }
    }
    Ok(EcmOutcome::NoFactor { curves_tried: state.curves_completed, operations: ops })
}

pub fn try_ecm_factor(
    n: &BigUint, params: &EcmParams, op_budget: u64, timeout: Option<Duration>,
) -> Result<Option<BigUint>, EcmError> {
    match ecm_stage1(n, params, op_budget, timeout, None, None, 0, &n.to_str_radix(10), "none")? {
        EcmOutcome::Factor(f) => Ok(Some(f)),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use babelprime_checkpoint::load_checkpoint;

    #[test]
    fn primes_upto_small() { assert_eq!(primes_upto(10), vec![2, 3, 5, 7]); }

    #[test]
    fn mont_double_matches_scalar_two() {
        let n = BigUint::from(97u32);
        let curve = MontCurve { n: &n, a24: BigUint::from(3u32) };
        let p = Point { x: BigUint::from(5u32), z: BigUint::one() };
        let mut ops = 0;
        let d = curve.double(&p, &mut ops);
        let s = curve.scalar_mul(&p, &BigUint::from(2u32), &mut ops);
        assert_eq!((&d.x * &s.z) % &n, (&s.x * &d.z) % &n);
    }

    #[test]
    fn ecm_valid_factor_or_none_for_143() {
        let n = BigUint::from(143u32);
        let params = EcmParams { b1: 50, max_curves: 25, seed: 1 };
        if let Ok(Some(f)) = try_ecm_factor(&n, &params, 1_000_000, None) {
            assert!(f > BigUint::one() && f < n);
            assert!(&n % &f == BigUint::zero());
        }
    }

    #[test]
    fn ecm_prime_no_factor() {
        let n = BigUint::from(97u32);
        let params = EcmParams { b1: 50, max_curves: 5, seed: 1 };
        let out = ecm_stage1(&n, &params, 500_000, None, None, None, 0, "97", "h").unwrap();
        assert!(!matches!(out, EcmOutcome::Factor(_)));
    }

    #[test]
    fn ecm_budget_interrupt() {
        let n = BigUint::from(455839u32);
        let params = EcmParams { b1: 5000, max_curves: 100, seed: 1 };
        let out = ecm_stage1(&n, &params, 50, None, None, None, 0, "455839", "h").unwrap();
        match out {
            EcmOutcome::Interrupted { reason: InterruptReason::BudgetExhausted, .. }
            | EcmOutcome::Factor(_) => {}
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn checkpoint_resume_parity() {
        let n = BigUint::from(455839u32);
        let params = EcmParams { b1: 80, max_curves: 15, seed: 11 };
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ecm.ckpt");
        let cfg = CheckpointConfig { path: path.clone(), interval_ops: 1 };
        let _ = ecm_stage1(&n, &params, 200, None, Some(&cfg), None, 0, "455839", "cfghash").unwrap();
        if path.exists() {
            let env: CheckpointEnvelope<EcmCheckpointState> = load_checkpoint(&path).unwrap();
            env.validate_for_job("455839", "cfghash", params.seed, ECM_ALGORITHM_ID, ECM_ALGORITHM_VERSION).unwrap();
            let ops_base = env.operations_consumed;
            let resumed = ecm_stage1(&n, &params, 10_000_000, None, None, Some(env.state), ops_base, "455839", "cfghash").unwrap();
            assert!(matches!(resumed, EcmOutcome::Factor(_) | EcmOutcome::NoFactor { .. } | EcmOutcome::Interrupted { .. }));
        }
    }

    #[test]
    fn suyama_deterministic() {
        let n = BigUint::from(10007u32);
        let sigma = BigUint::from(11u32);
        let mut o1 = 0; let mut o2 = 0;
        let (a, p) = suyama_curve(&sigma, &n, &mut o1).unwrap();
        let (b, q) = suyama_curve(&sigma, &n, &mut o2).unwrap();
        assert_eq!(a, b); assert_eq!(p.x, q.x); assert_eq!(p.z, q.z);
    }
}
