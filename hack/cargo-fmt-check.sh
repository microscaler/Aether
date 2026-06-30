#!/usr/bin/env bash
# Run cargo fmt --check on the workspace.
set -euo pipefail
cargo fmt -- --check
