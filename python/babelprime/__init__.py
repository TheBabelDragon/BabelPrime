"""BabelPrime Python research API (v0.1).

Thin convenience layer over the Rust core. For production verification
prefer calling the Rust library or CLI directly.
"""

from __future__ import annotations

import json
import subprocess
import shutil
from typing import Any, Dict, Optional

__version__ = "0.1.0"


def _find_cli() -> Optional[str]:
    return shutil.which("babelprime")


def factor(n: str | int, seed: int = 0xB4B31, timeout_ms: int = 30000) -> Dict[str, Any]:
    """Factor an integer and return the result document as a dict.

    Requires the `babelprime` CLI to be on PATH (built from the Rust workspace).
    """
    cli = _find_cli()
    if cli is None:
        raise RuntimeError(
            "babelprime CLI not found on PATH. Build with `cargo build -p babelprime-api --release` "
            "and ensure target/release is on PATH."
        )
    cmd = [
        cli,
        "factor",
        str(n),
        "--seed",
        str(seed),
        "--timeout-ms",
        str(timeout_ms),
        "--json",
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr or proc.stdout or "factorization failed")
    return json.loads(proc.stdout)


def primality(n: str | int, rounds: int = 16, seed: int = 42) -> Dict[str, Any]:
    """Run Miller–Rabin and return evidence as a dict."""
    cli = _find_cli()
    if cli is None:
        raise RuntimeError("babelprime CLI not found on PATH")
    cmd = [cli, "primality", str(n), "--rounds", str(rounds), "--seed", str(seed)]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr or "primality test failed")
    return json.loads(proc.stdout)
