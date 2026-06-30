#!/usr/bin/env bash
# Run clippy on the entire workspace, treating warnings as errors.
set -euo pipefail
cargo clippy --workspace --all-targets -- -D warnings
