#!/usr/bin/env bash
# Run cargo nextest on the workspace.
set -euo pipefail
cargo nextest run --workspace --all-targets
