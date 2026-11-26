//! Factorization algorithms and job engine for BabelPrime.

use babelprime_arithmetic::{
    bit_length, gcd, mod_pow, parse_integer, perfect_power, ArithmeticError, CanonicalInteger,
};
use babelprime_primality::{miller_rabin, PrimalityStatus};
use num_bigint::BigUint;
use num_traits::{One, Zero};
use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};
use thiserror::Error;

pub const SCHEMA_VERSION: &str = "0.1.0";
pub const ALGORITHM_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Completeness {
    Complete,
    Partial,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    Verified,
    Failed,
    Partial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorPower {
    pub prime: CanonicalInteger,
    pub exponent: u32,
    pub primality: PrimalityStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub algorithms_used: Vec<String>,
    pub input_bit_length: u64,
    pub elapsed_ms: u64,
    pub operations_estimate: u64,
    pub budget_exhausted: bool,
    pub timed_out: bool,
    pub cancelled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReproducibilityMetadata {
    pub seed: u64,
    /// Hash of the full normalized configuration (not wall-clock).
    pub config_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorizationResult {
    pub schema_version: String,
    pub algorithm_version: String,
    pub input_integer: CanonicalInteger,
    pub discovered_factors: Vec<FactorPower>,
    pub primality_status: PrimalityStatus,
    pub verification_status: VerificationStatus,
    pub completeness: Completeness,
    pub execution_metadata: ExecutionMetadata,
    pub reproducibility_metadata: ReproducibilityMetadata,
    pub remaining_cofactor: Option<CanonicalInteger>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorJobConfig {
    pub seed: u64,
    pub timeout_ms: u64,
    pub operation_budget: u64,
    pub mr_rounds: u32,
    pub trial_bound: u64,
    pub enable_rho: bool,
    pub enable_p1: bool,
    pub max_depth: u32,
}

impl Default for FactorJobConfig {
    fn default() -> Self {
        Self {
            seed: 0xB4B31,
            timeout_ms: 30_000,
            operation_budget: 10_000_000,
            mr_rounds: 16,
            trial_bound: 100_000,
            enable_rho: true,
            enable_p1: true,
            max_depth: 32,
        }
    }
}

impl FactorJobConfig {
    /// Platform-stable identity of the full normalized configuration.
    /// Uses FNV-1a 64 over a canonical field encoding (not DefaultHasher).
    pub fn config_hash(&self) -> String {
        let encoded = format!(
            "v={}|seed={}|timeout={}|budget={}|mr={}|trial={}|rho={}|p1={}|depth={}",
            ALGORITHM_VERSION,
            self.seed,
            self.timeout_ms,
            self.operation_budget,
            self.mr_rounds,
            self.trial_bound,
            self.enable_rho as u8,
            self.enable_p1 as u8,
            self.max_depth,
        );
        format!("{:016x}", fnv1a64(encoded.as_bytes()))
    }
}

fn fnv1a64(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in data {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[derive(Debug, Error)]
pub enum FactorError {
    #[error(transparent)]
    Arithmetic(#[from] ArithmeticError),
    #[error("timeout")]
    Timeout,
    #[error("budget exhausted")]
    BudgetExhausted,
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

struct State {
    original: BigUint,
    remaining: BigUint,
    factors: BTreeMap<BigUint, u32>,
    ops: u64,
    depth: u32,
    algorithms: Vec<String>,
}

impl State {
    fn new(n: BigUint) -> Self {
        Self {
            original: n.clone(),
            remaining: n,
            factors: BTreeMap::new(),
            ops: 0,
            depth: 0,
            algorithms: Vec::new(),
        }
    }
    fn record(&mut self, name: &str) {
        if !self.algorithms.iter().any(|a| a == name) {
            self.algorithms.push(name.to_string());
        }
    }
}

pub fn factorize(
    n: &BigUint,
    config: &FactorJobConfig,
) -> Result<FactorizationResult, FactorError> {
    if n < &BigUint::from(2u32) {
        return Err(FactorError::InvalidInput("input must be >= 2".into()));
    }
    if config.mr_rounds == 0 {
        return Err(FactorError::InvalidInput("mr_rounds must be >= 1".into()));
    }
    let start = Instant::now();
    let deadline =
        (config.timeout_ms > 0).then(|| start + Duration::from_millis(config.timeout_ms));
    let mut st = State::new(n.clone());

    while !st.remaining.bit(0) {
        check(&st, config, deadline)?;
        *st.factors.entry(2u32.into()).or_insert(0) += 1;
        st.remaining >>= 1u32;
        st.ops += 1;
        st.record("trial_division");
    }

    let outcome = if st.remaining.is_one() { Ok(()) } else { decompose(&mut st, config, deadline) };
    let (timed_out, budget) = match outcome {
        Ok(()) => (false, false),
        Err(FactorError::Timeout) => (true, false),
        Err(FactorError::BudgetExhausted) => (false, true),
        Err(e) => return Err(e),
    };
    finalize(st, config, start, timed_out, budget)
}

fn check(
    st: &State,
    config: &FactorJobConfig,
    deadline: Option<Instant>,
) -> Result<(), FactorError> {
    if let Some(dl) = deadline {
        if Instant::now() >= dl {
            return Err(FactorError::Timeout);
        }
    }
    if config.operation_budget > 0 && st.ops >= config.operation_budget {
        return Err(FactorError::BudgetExhausted);
    }
    Ok(())
}

fn decompose(
    st: &mut State,
    config: &FactorJobConfig,
    deadline: Option<Instant>,
) -> Result<(), FactorError> {
    check(st, config, deadline)?;
    if st.remaining.is_one() {
        return Ok(());
    }
    if st.depth >= config.max_depth {
        return Err(FactorError::BudgetExhausted);
    }

    st.ops += 10;
    check(st, config, deadline)?;
    if let Some((base, exp)) = perfect_power(&st.remaining) {
        st.record("perfect_power");
        st.ops += 40;
        let mut sub = State::new(base);
        sub.depth = st.depth + 1;
        sub.ops = st.ops;
        decompose(&mut sub, config, deadline)?;
        for (p, e) in sub.factors {
            *st.factors.entry(p).or_insert(0) += e * exp;
        }
        st.remaining = BigUint::one();
        st.ops = sub.ops;
        for a in sub.algorithms {
            st.record(&a);
        }
        return Ok(());
    }

    let ev = miller_rabin(&st.remaining, config.mr_rounds, config.seed)
        .map_err(|e| FactorError::Arithmetic(ArithmeticError::InvalidInput(e.to_string())))?;
    st.ops += config.mr_rounds as u64 * 5;
    check(st, config, deadline)?;
    match ev.status {
        PrimalityStatus::ProvenPrime | PrimalityStatus::ProbablePrime { .. } => {
            *st.factors.entry(st.remaining.clone()).or_insert(0) += 1;
            st.remaining = BigUint::one();
            st.record("miller_rabin");
            return Ok(());
        }
        _ => {}
    }

    if let Some(f) = trial(&st.remaining, config.trial_bound, &mut st.ops, config, deadline)? {
        st.record("trial_division");
        divide_out(st, &f, config, deadline)?;
        return decompose(st, config, deadline);
    }

    if config.enable_rho {
        if let Some(f) =
            rho(&st.remaining, config.seed.wrapping_add(st.ops), &mut st.ops, config, deadline)?
        {
            st.record("pollard_rho");
            divide_out(st, &f, config, deadline)?;
            return decompose(st, config, deadline);
        }
    }

    if config.enable_p1 {
        if let Some(f) = p1(&st.remaining, config.seed, &mut st.ops, config, deadline)? {
            st.record("pollard_p1");
            divide_out(st, &f, config, deadline)?;
            return decompose(st, config, deadline);
        }
    }

    Ok(())
}

fn divide_out(
    st: &mut State,
    f: &BigUint,
    config: &FactorJobConfig,
    deadline: Option<Instant>,
) -> Result<(), FactorError> {
    if f.is_one() || f >= &st.remaining {
        return Ok(());
    }
    while &st.remaining % f == BigUint::zero() {
        check(st, config, deadline)?;
        *st.factors.entry(f.clone()).or_insert(0) += 1;
        st.remaining /= f;
        st.ops += 1;
    }
    Ok(())
}

fn trial(
    n: &BigUint,
    bound: u64,
    ops: &mut u64,
    config: &FactorJobConfig,
    deadline: Option<Instant>,
) -> Result<Option<BigUint>, FactorError> {
    let mut d = BigUint::from(3u32);
    let limit = BigUint::from(bound);
    let upper = n.sqrt() + BigUint::one();
    let upper = if limit < upper { limit } else { upper };
    while d <= upper {
        if config.operation_budget > 0 && *ops >= config.operation_budget {
            return Err(FactorError::BudgetExhausted);
        }
        if let Some(dl) = deadline {
            if Instant::now() >= dl {
                return Err(FactorError::Timeout);
            }
        }
        *ops += 1;
        if n % &d == BigUint::zero() {
            return Ok(Some(d));
        }
        d += 2u32;
    }
    Ok(None)
}

fn rho(
    n: &BigUint,
    seed: u64,
    ops: &mut u64,
    config: &FactorJobConfig,
    deadline: Option<Instant>,
) -> Result<Option<BigUint>, FactorError> {
    if !n.bit(0) {
        return Ok(Some(2u32.into()));
    }
    let mut rng = ChaCha20Rng::seed_from_u64(seed);
    for _attempt in 0u64..4 {
        let c = BigUint::from(rng.gen_range(1u64..1000));
        let mut x = BigUint::from(rng.gen_range(2u64..10_000));
        let mut y = x.clone();
        let mut d = BigUint::one();
        let f = |x: &BigUint| (x * x + &c) % n;
        let max = (config.operation_budget / 10).clamp(1_000, 100_000);
        let mut steps = 0u64;
        while d.is_one() && steps < max {
            if config.operation_budget > 0 && *ops >= config.operation_budget {
                return Err(FactorError::BudgetExhausted);
            }
            if let Some(dl) = deadline {
                if Instant::now() >= dl {
                    return Err(FactorError::Timeout);
                }
            }
            x = f(&x);
            y = f(&f(&y));
            let diff = if x > y { &x - &y } else { &y - &x };
            d = gcd(&diff, n);
            *ops += 3;
            steps += 1;
        }
        if !d.is_one() && &d != n {
            return Ok(Some(d));
        }
    }
    Ok(None)
}

fn p1(
    n: &BigUint,
    seed: u64,
    ops: &mut u64,
    config: &FactorJobConfig,
    deadline: Option<Instant>,
) -> Result<Option<BigUint>, FactorError> {
    let mut rng = ChaCha20Rng::seed_from_u64(seed ^ 0x5041);
    let a0 = if n > &BigUint::from(4u32) {
        let range = n - BigUint::from(3u32);
        let mut bytes = vec![0u8; (range.bits() as usize / 8) + 2];
        rng.fill_bytes(&mut bytes);
        BigUint::from_bytes_be(&bytes) % &range + BigUint::from(2u32)
    } else {
        BigUint::from(2u32)
    };
    let mut a = a0 % n;
    if a.is_zero() || a.is_one() {
        a = BigUint::from(2u32);
    }
    let bound = config.trial_bound.clamp(1000, 50_000);
    for i in 2..=bound {
        if config.operation_budget > 0 && *ops >= config.operation_budget {
            return Err(FactorError::BudgetExhausted);
        }
        if let Some(dl) = deadline {
            if Instant::now() >= dl {
                return Err(FactorError::Timeout);
            }
        }
        a = mod_pow(&a, &BigUint::from(i), n)?;
        if a.is_zero() {
            continue;
        }
        let am1 = &a - BigUint::one();
        let g = gcd(&am1, n);
        *ops += 5;
        if g > BigUint::one() && &g < n {
            return Ok(Some(g));
        }
    }
    Ok(None)
}

fn is_prime_quick(n: &BigUint, config: &FactorJobConfig) -> bool {
    miller_rabin(n, config.mr_rounds.clamp(1, 8), config.seed)
        .map(|e| {
            matches!(e.status, PrimalityStatus::ProvenPrime | PrimalityStatus::ProbablePrime { .. })
        })
        .unwrap_or(false)
}

fn finalize(
    st: State,
    config: &FactorJobConfig,
    start: Instant,
    timed_out: bool,
    budget: bool,
) -> Result<FactorizationResult, FactorError> {
    let mut clean: BTreeMap<BigUint, u32> = BTreeMap::new();
    let mut leftover = st.remaining.clone();
    for (f, e) in st.factors {
        if is_prime_quick(&f, config) {
            *clean.entry(f).or_insert(0) += e;
        } else {
            for _ in 0..e {
                leftover *= &f;
            }
        }
    }

    let mut discovered = Vec::new();
    for (p, e) in &clean {
        let status = miller_rabin(p, config.mr_rounds, config.seed)
            .map(|e| e.status)
            .unwrap_or(PrimalityStatus::Undetermined);
        discovered.push(FactorPower {
            prime: CanonicalInteger::from_biguint(p),
            exponent: *e,
            primality: status,
        });
    }
    discovered.sort_by(|a, b| a.prime.0.cmp(&b.prime.0));

    let mut product = BigUint::one();
    for (p, e) in &clean {
        for _ in 0..*e {
            product *= p;
        }
    }
    if !leftover.is_one() {
        product *= &leftover;
    }

    let completeness = if leftover.is_one() && product == st.original {
        Completeness::Complete
    } else if !clean.is_empty() {
        Completeness::Partial
    } else {
        Completeness::Incomplete
    };

    let verification = if product == st.original {
        if completeness == Completeness::Complete {
            VerificationStatus::Verified
        } else {
            VerificationStatus::Partial
        }
    } else {
        VerificationStatus::Failed
    };

    let input_primality = if completeness == Completeness::Complete && clean.len() == 1 {
        let (p, e) = clean.iter().next().unwrap();
        if *e == 1 {
            miller_rabin(p, config.mr_rounds, config.seed)
                .map(|e| e.status)
                .unwrap_or(PrimalityStatus::Undetermined)
        } else {
            PrimalityStatus::Composite
        }
    } else if completeness == Completeness::Complete {
        PrimalityStatus::Composite
    } else {
        PrimalityStatus::Undetermined
    };

    Ok(FactorizationResult {
        schema_version: SCHEMA_VERSION.into(),
        algorithm_version: ALGORITHM_VERSION.into(),
        input_integer: CanonicalInteger::from_biguint(&st.original),
        discovered_factors: discovered,
        primality_status: input_primality,
        verification_status: verification,
        completeness,
        execution_metadata: ExecutionMetadata {
            algorithms_used: st.algorithms,
            input_bit_length: bit_length(&st.original),
            elapsed_ms: start.elapsed().as_millis() as u64,
            operations_estimate: st.ops,
            budget_exhausted: budget,
            timed_out,
            cancelled: false,
        },
        reproducibility_metadata: ReproducibilityMetadata {
            seed: config.seed,
            config_hash: config.config_hash(),
        },
        remaining_cofactor: if leftover.is_one() {
            None
        } else {
            Some(CanonicalInteger::from_biguint(&leftover))
        },
    })
}

pub fn factorize_str(
    s: &str,
    config: Option<FactorJobConfig>,
) -> Result<FactorizationResult, FactorError> {
    let n = parse_integer(s)?;
    factorize(&n, &config.unwrap_or_default())
}

pub mod future {
    pub fn ecm_available() -> bool {
        false
    }
    pub fn quadratic_sieve_available() -> bool {
        false
    }
    pub fn gnfs_available() -> bool {
        false
    }
    pub fn quantum_order_finding_available() -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factor_15() {
        let res = factorize(&15u32.into(), &FactorJobConfig::default()).unwrap();
        assert_eq!(res.completeness, Completeness::Complete);
        assert_eq!(res.verification_status, VerificationStatus::Verified);
    }

    #[test]
    fn factor_251_squared() {
        let n = BigUint::from(63001u32);
        let res = factorize(&n, &FactorJobConfig::default()).unwrap();
        assert_eq!(res.completeness, Completeness::Complete);
        assert_eq!(res.discovered_factors.len(), 1);
        assert_eq!(res.discovered_factors[0].prime.as_str(), "251");
        assert_eq!(res.discovered_factors[0].exponent, 2);
    }

    #[test]
    fn factor_prime() {
        let res = factorize(&17u32.into(), &FactorJobConfig::default()).unwrap();
        assert_eq!(res.discovered_factors.len(), 1);
    }

    #[test]
    fn factor_8() {
        let res = factorize(&8u32.into(), &FactorJobConfig::default()).unwrap();
        assert_eq!(res.discovered_factors[0].exponent, 3);
    }

    #[test]
    fn zero_mr_rounds_rejected() {
        let cfg = FactorJobConfig { mr_rounds: 0, ..Default::default() };
        assert!(factorize(&15u32.into(), &cfg).is_err());
    }

    #[test]
    fn config_hash_stable() {
        let a = FactorJobConfig::default();
        let b = FactorJobConfig::default();
        assert_eq!(a.config_hash(), b.config_hash());
        let c = FactorJobConfig { seed: 99, ..Default::default() };
        assert_ne!(a.config_hash(), c.config_hash());
    }
}
