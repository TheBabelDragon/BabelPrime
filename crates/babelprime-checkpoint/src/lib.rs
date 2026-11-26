//! Durable checkpoint/resume infrastructure for BabelPrime.
//!
//! # Integrity vs authenticity
//!
//! Checkpoints carry a **SHA-256** digest over the envelope (excluding the digest
//! field itself). This detects accidental corruption and truncated writes.
//! It is **not** a keyed MAC: it does not authenticate the writer or protect
//! against an adversary who can rewrite both payload and digest.
//!
//! Restoring a checkpoint does not change mathematical results. Operation
//! accounting is restored from the checkpoint and continues upward.

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

pub const CHECKPOINT_SCHEMA_VERSION: &str = "0.2.0";
pub const DEFAULT_ALGORITHM_VERSION: &str = "0.2.0";

#[derive(Debug, Error)]
pub enum CheckpointError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("integrity check failed (SHA-256): expected {expected}, got {actual}")]
    Integrity { expected: String, actual: String },
    #[error("unsupported checkpoint schema version: {0}")]
    UnsupportedVersion(String),
    #[error("checkpoint mismatch: {0}")]
    Mismatch(String),
    #[error("checkpoint truncated or empty")]
    Truncated,
    #[error("malformed checkpoint payload: {0}")]
    Malformed(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointEnvelope<T> {
    pub schema_version: String,
    pub algorithm_version: String,
    pub input_integer: String,
    pub config_hash: String,
    pub seed: u64,
    pub operations_consumed: u64,
    pub algorithm: String,
    pub state: T,
    pub integrity_hash: String,
}

impl<T: Serialize + DeserializeOwned> CheckpointEnvelope<T> {
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
        Ok(hex::encode(Sha256::digest(&bytes)))
    }

    pub fn validate_integrity(&self) -> Result<(), CheckpointError> {
        if self.schema_version != CHECKPOINT_SCHEMA_VERSION {
            return Err(CheckpointError::UnsupportedVersion(self.schema_version.clone()));
        }
        let actual = self.compute_hash()?;
        if actual != self.integrity_hash {
            return Err(CheckpointError::Integrity {
                expected: self.integrity_hash.clone(),
                actual,
            });
        }
        Ok(())
    }

    pub fn validate_for_job(
        &self,
        input_integer: &str,
        config_hash: &str,
        seed: u64,
        algorithm: &str,
        algorithm_version: &str,
    ) -> Result<(), CheckpointError> {
        self.validate_integrity()?;
        if self.input_integer != input_integer {
            return Err(CheckpointError::Mismatch(format!("input mismatch")));
        }
        if self.config_hash != config_hash {
            return Err(CheckpointError::Mismatch(format!("config_hash mismatch")));
        }
        if self.seed != seed {
            return Err(CheckpointError::Mismatch(format!("seed mismatch")));
        }
        if self.algorithm != algorithm {
            return Err(CheckpointError::Mismatch(format!("algorithm mismatch")));
        }
        if self.algorithm_version != algorithm_version {
            return Err(CheckpointError::Mismatch(format!("algorithm_version mismatch")));
        }
        Ok(())
    }
}

pub fn save_checkpoint<T: Serialize>(
    path: &Path,
    envelope: &CheckpointEnvelope<T>,
) -> Result<(), CheckpointError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let tmp = unique_tmp_path(path);
    let result = (|| {
        let mut f = File::create(&tmp)?;
        f.write_all(&serde_json::to_vec_pretty(envelope)?)?;
        f.sync_all()?;
        fs::rename(&tmp, path)?;
        Ok::<(), CheckpointError>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
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
    let env: CheckpointEnvelope<T> = serde_json::from_slice(&buf).map_err(|e| {
        if buf.contains(&b'{') && !buf.ends_with(b"}") {
            CheckpointError::Truncated
        } else {
            CheckpointError::Serde(e)
        }
    })?;
    env.validate_integrity()?;
    Ok(env)
}

fn unique_tmp_path(path: &Path) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id();
    let mut s = path.as_os_str().to_os_string();
    s.push(format!(".{pid}.{nanos}.tmp"));
    PathBuf::from(s)
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
            DummyState { step, label: "ok".into() },
        )
        .unwrap()
    }

    #[test]
    fn round_trip_integrity() {
        let env = sample_env(3);
        env.validate_integrity().unwrap();
        assert_eq!(env.integrity_hash.len(), 64);
        let restored: CheckpointEnvelope<DummyState> =
            serde_json::from_slice(&serde_json::to_vec(&env).unwrap()).unwrap();
        restored.validate_integrity().unwrap();
        assert_eq!(restored.state.step, 3);
    }

    #[test]
    fn reject_tampered_payload() {
        let env = sample_env(1);
        let mut v = serde_json::to_value(&env).unwrap();
        v["state"]["step"] = json!(999);
        let tampered: CheckpointEnvelope<DummyState> = serde_json::from_value(v).unwrap();
        assert!(matches!(tampered.validate_integrity(), Err(CheckpointError::Integrity { .. })));
    }

    #[test]
    fn reject_version_mismatch() {
        let mut env = sample_env(1);
        env.schema_version = "9.9.9".into();
        env.integrity_hash = env.compute_hash().unwrap();
        assert!(matches!(env.validate_integrity(), Err(CheckpointError::UnsupportedVersion(_))));
    }

    #[test]
    fn reject_algorithm_version_mismatch() {
        let env = sample_env(1);
        assert!(matches!(
            env.validate_for_job("91", "deadbeef", 42, "dummy", "0.1.0"),
            Err(CheckpointError::Mismatch(_))
        ));
    }

    #[test]
    fn atomic_save_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("job.ckpt");
        save_checkpoint(&path, &sample_env(7)).unwrap();
        let loaded: CheckpointEnvelope<DummyState> = load_checkpoint(&path).unwrap();
        assert_eq!(loaded.state.step, 7);
        assert_eq!(loaded.operations_consumed, 70);
    }

    #[test]
    fn reject_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.ckpt");
        File::create(&path).unwrap();
        assert!(matches!(load_checkpoint::<DummyState>(&path), Err(CheckpointError::Truncated)));
    }

    #[test]
    fn deterministic_hash() {
        assert_eq!(sample_env(5).integrity_hash, sample_env(5).integrity_hash);
    }
}
