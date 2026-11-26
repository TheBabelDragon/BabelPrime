//! Durable checkpoint/resume infrastructure for BabelPrime.
//!
//! Checkpoints are versioned JSON documents with integrity hashes.
//! Persistence is atomic: write to a temporary file, flush, then rename.
//!
//! Restoring a checkpoint does **not** change mathematical results: the same
//! input, configuration hash, and seed yield an equivalent factorization
//! outcome whether the job ran uninterrupted or was resumed.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Checkpoint schema version. Bump when the envelope fields change.
pub const CHECKPOINT_SCHEMA_VERSION: &str = "0.2.0";

/// Algorithm-version string embedded in checkpoints for compatibility checks.
pub const DEFAULT_ALGORITHM_VERSION: &str = "0.2.0";

#[derive(Debug, Error)]
pub enum CheckpointError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("integrity check failed: expected hash {expected}, got {actual}")]
    Integrity { expected: String, actual: String },
    #[error("unsupported checkpoint schema version: {0}")]
    UnsupportedVersion(String),
    #[error("checkpoint mismatch: {0}")]
    Mismatch(String),
    #[error("checkpoint truncated or empty")]
    Truncated,
    #[error("checkpoint path required")]
    MissingPath,
}

/// Envelope shared by all algorithm-specific checkpoint payloads.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointEnvelope<T> {
    pub schema_version: String,
    pub algorithm_version: String,
    /// Canonical decimal input integer.
    pub input_integer: String,
    /// Stable FNV-1a (or equivalent) of the normalized job configuration.
    pub config_hash: String,
    pub seed: u64,
    /// Cumulative operation estimate at the time of the checkpoint.
    pub operations_consumed: u64,
    /// Algorithm identifier (e.g. "ecm_stage1").
    pub algorithm: String,
    /// Algorithm-specific state.
    pub state: T,
    /// Integrity hash over the canonical serialization of all fields except `integrity_hash`.
    pub integrity_hash: String,
}

impl<T: Serialize + DeserializeOwned> CheckpointEnvelope<T> {
    /// Build an envelope and compute its integrity hash.
    pub fn new(
        algorithm_version: &str,
        input_integer: &str,
        config_hash: &str,
        seed: u64,
        operations_consumed: u64,
        algorithm: &str,
        state: T,
    ) -> Result<Self, CheckpointError> {
        let mut env = Self {
            schema_version: CHECKPOINT_SCHEMA_VERSION.into(),
            algorithm_version: algorithm_version.into(),
            input_integer: input_integer.into(),
            config_hash: config_hash.into(),
            seed,
            operations_consumed,
            algorithm: algorithm.into(),
            state,
            integrity_hash: String::new(),
        };
        env.integrity_hash = env.compute_hash()?;
        Ok(env)
    }

    fn payload_for_hash(&self) -> Result<Vec<u8>, CheckpointError> {
        let mut clone = serde_json::to_value(self)?;
        if let Some(obj) = clone.as_object_mut() {
            obj.remove("integrity_hash");
        }
        Ok(serde_json::to_vec(&clone)?)
    }

    pub fn compute_hash(&self) -> Result<String, CheckpointError> {
        let bytes = self.payload_for_hash()?;
        Ok(format!("{:016x}", fnv1a64(&bytes)))
    }

    pub fn validate_integrity(&self) -> Result<(), CheckpointError> {
        let actual = self.compute_hash()?;
        if actual != self.integrity_hash {
            return Err(CheckpointError::Integrity {
                expected: self.integrity_hash.clone(),
                actual,
            });
        }
        if self.schema_version != CHECKPOINT_SCHEMA_VERSION {
            return Err(CheckpointError::UnsupportedVersion(self.schema_version.clone()));
        }
        Ok(())
    }

    pub fn validate_for_job(
        &self,
        input_integer: &str,
        config_hash: &str,
        seed: u64,
        algorithm: &str,
    ) -> Result<(), CheckpointError> {
        self.validate_integrity()?;
        if self.input_integer != input_integer {
            return Err(CheckpointError::Mismatch(format!(
                "input: checkpoint {} vs expected {}",
                self.input_integer, input_integer
            )));
        }
        if self.config_hash != config_hash {
            return Err(CheckpointError::Mismatch(format!(
                "config_hash: checkpoint {} vs expected {}",
                self.config_hash, config_hash
            )));
        }
        if self.seed != seed {
            return Err(CheckpointError::Mismatch(format!(
                "seed: checkpoint {} vs expected {}",
                self.seed, seed
            )));
        }
        if self.algorithm != algorithm {
            return Err(CheckpointError::Mismatch(format!(
                "algorithm: checkpoint {} vs expected {}",
                self.algorithm, algorithm
            )));
        }
        Ok(())
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

pub fn save_checkpoint<T: Serialize>(
    path: &Path,
    envelope: &CheckpointEnvelope<T>,
) -> Result<(), CheckpointError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let tmp = tmp_path(path);
    {
        let mut f = File::create(&tmp)?;
        let data = serde_json::to_vec_pretty(envelope)?;
        f.write_all(&data)?;
        f.sync_all()?;
    }
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn load_checkpoint<T: Serialize + DeserializeOwned>(
    path: &Path,
) -> Result<CheckpointEnvelope<T>, CheckpointError> {
    let mut f = File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    if buf.is_empty() {
        return Err(CheckpointError::Truncated);
    }
    let env: CheckpointEnvelope<T> = serde_json::from_slice(&buf)?;
    env.validate_integrity()?;
    Ok(env)
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".tmp");
    PathBuf::from(tmp)
}

#[derive(Debug, Clone)]
pub struct CheckpointConfig {
    pub path: PathBuf,
    pub interval_ops: u64,
}

impl CheckpointConfig {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            interval_ops: 50_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    struct DummyState {
        step: u64,
        label: String,
    }

    fn sample_env(step: u64) -> CheckpointEnvelope<DummyState> {
        CheckpointEnvelope::new(
            DEFAULT_ALGORITHM_VERSION,
            "91",
            "deadbeef",
            42,
            step * 10,
            "dummy",
            DummyState {
                step,
                label: "ok".into(),
            },
        )
        .unwrap()
    }

    #[test]
    fn round_trip_integrity() {
        let env = sample_env(3);
        env.validate_integrity().unwrap();
        let bytes = serde_json::to_vec(&env).unwrap();
        let restored: CheckpointEnvelope<DummyState> = serde_json::from_slice(&bytes).unwrap();
        restored.validate_integrity().unwrap();
        assert_eq!(restored.state.step, 3);
    }

    #[test]
    fn reject_tampered_payload() {
        let env = sample_env(1);
        let mut v = serde_json::to_value(&env).unwrap();
        v["state"]["step"] = json!(999);
        let tampered: CheckpointEnvelope<DummyState> = serde_json::from_value(v).unwrap();
        assert!(matches!(
            tampered.validate_integrity(),
            Err(CheckpointError::Integrity { .. })
        ));
    }

    #[test]
    fn reject_version_mismatch() {
        let mut env = sample_env(1);
        env.schema_version = "9.9.9".into();
        env.integrity_hash = env.compute_hash().unwrap();
        assert!(matches!(
            env.validate_integrity(),
            Err(CheckpointError::UnsupportedVersion(_))
        ));
    }

    #[test]
    fn reject_job_mismatch() {
        let env = sample_env(1);
        let err = env.validate_for_job("15", "deadbeef", 42, "dummy");
        assert!(matches!(err, Err(CheckpointError::Mismatch(_))));
    }

    #[test]
    fn atomic_save_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("job.ckpt");
        let env = sample_env(7);
        save_checkpoint(&path, &env).unwrap();
        let loaded: CheckpointEnvelope<DummyState> = load_checkpoint(&path).unwrap();
        assert_eq!(loaded.state.step, 7);
        loaded
            .validate_for_job("91", "deadbeef", 42, "dummy")
            .unwrap();
    }

    #[test]
    fn reject_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.ckpt");
        File::create(&path).unwrap();
        let err = load_checkpoint::<DummyState>(&path);
        assert!(matches!(err, Err(CheckpointError::Truncated)));
    }

    #[test]
    fn reject_truncated_json() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trunc.ckpt");
        fs::write(&path, b"{\"schema_version\":\"0.2.0\"").unwrap();
        assert!(load_checkpoint::<DummyState>(&path).is_err());
    }

    #[test]
    fn deterministic_hash() {
        let a = sample_env(5);
        let b = sample_env(5);
        assert_eq!(a.integrity_hash, b.integrity_hash);
    }
}
