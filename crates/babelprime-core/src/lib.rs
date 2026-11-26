//! BabelPrime computational core.

pub use babelprime_arithmetic::{
    bit_length, gcd, mod_pow, parse_integer, perfect_power, CanonicalInteger, ArithmeticError,
};
pub use babelprime_factor::{
    factorize, factorize_str, Completeness, ExecutionMetadata, FactorJobConfig, FactorPower,
    FactorizationResult, ReproducibilityMetadata, VerificationStatus, ALGORITHM_VERSION,
    SCHEMA_VERSION,
};
pub use babelprime_primality::{
    is_probable_prime, miller_rabin, PrimalityEvidence, PrimalityStatus,
};
pub use babelprime_verify::{recheck_primality, verify_json, verify_result, VerifyError};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn factor_to_json(input: &str, config: Option<FactorJobConfig>) -> Result<String, String> {
    let res = factorize_str(input, config).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&res).map_err(|e| e.to_string())
}

pub fn primality_to_json(input: &str, rounds: u32, seed: u64) -> Result<String, String> {
    let n = parse_integer(input).map_err(|e| e.to_string())?;
    let ev = miller_rabin(&n, rounds, seed).map_err(|e| e.to_string())?;
    serde_json::to_string_pretty(&ev).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_to_end_json() {
        let json = factor_to_json("143", None).unwrap();
        assert!(json.contains("schema_version"));
        let res: FactorizationResult = serde_json::from_str(&json).unwrap();
        assert!(verify_result(&res).is_ok());
    }
}
