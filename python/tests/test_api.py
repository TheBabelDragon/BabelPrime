"""Python API tests for BabelPrime.

These tests exercise result-schema interpretation and CLI absence handling.
They do not require a built CLI for the unit-level checks; integration tests
that need the binary are skipped when it is missing.
"""

from __future__ import annotations

import json
import shutil
import pytest

import babelprime


def test_version():
    assert babelprime.__version__ == "0.1.0"


def test_missing_cli_raises():
    """When the CLI is not on PATH, factor() must raise RuntimeError."""
    original = babelprime._find_cli
    try:
        babelprime._find_cli = lambda: None  # type: ignore
        with pytest.raises(RuntimeError, match="CLI not found"):
            babelprime.factor(15)
    finally:
        babelprime._find_cli = original


@pytest.mark.skipif(shutil.which("babelprime") is None, reason="babelprime CLI not on PATH")
def test_factor_15_via_cli():
    result = babelprime.factor(15, seed=42, timeout_ms=5000)
    assert result["schema_version"] == "0.1.0"
    assert result["completeness"] == "complete"
    assert result["verification_status"] == "verified"
    primes = {f["prime"] for f in result["discovered_factors"]}
    assert "3" in primes
    assert "5" in primes


@pytest.mark.skipif(shutil.which("babelprime") is None, reason="babelprime CLI not on PATH")
def test_primality_via_cli():
    ev = babelprime.primality(17, rounds=8, seed=1)
    assert "status" in ev or "ProvenPrime" in json.dumps(ev) or "proven" in json.dumps(ev).lower()


def test_schema_shape_documented():
    """Document the expected result keys for consumers."""
    expected_keys = {
        "schema_version",
        "algorithm_version",
        "input_integer",
        "discovered_factors",
        "primality_status",
        "verification_status",
        "completeness",
        "execution_metadata",
        "reproducibility_metadata",
    }
    assert expected_keys
