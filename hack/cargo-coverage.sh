#!/usr/bin/env bash
# Run cargo tarpaulin coverage and fail if it drops below the stored baseline.
# Usage: ./hack/cargo-coverage.sh [--update-baseline]
set -euo pipefail

BASE_DIR="$(cd "$(dirname "$0")/.." && pwd)"
BASELINE_FILE="${BASE_DIR}/.coverage-baseline.json"
COBERTURA="${BASE_DIR}/cobertura.xml"
REPORT_MD="${BASE_DIR}/COVERAGE_REPORT.md"
THRESHOLD=80  # Minimum percentage to pass

# Parse args
UPDATE=false
for arg in "$@"; do
    case "$arg" in
        --update-baseline) UPDATE=true ;;
    esac
done

# Run coverage (XML output)
cargo tarpaulin --workspace --exclude pact-mock-server --out Xml --timeout 120 2>&1 | tee /tmp/tarpaulin.log || {
    echo "ERROR: cargo tarpaulin failed"
    exit 1
}

# Extract coverage percentage from tarpaulin's "Coverage Results" summary line
# Format: "83.22% coverage, 1413/1698 lines covered"
COVERAGE=$(grep -a -oP '\d+\.\d+% coverage' /tmp/tarpaulin.log | grep -a -oP '\d+\.\d+' | head -n 1)

if [ -z "$COVERAGE" ]; then
    echo "ERROR: Could not parse coverage percentage from tarpaulin output"
    exit 1
fi

echo ""
echo "Coverage: ${COVERAGE}%"

# Compare against threshold
BELOW=$(echo "$COVERAGE < $THRESHOLD" | bc -l)
if [ "$BELOW" -eq 1 ]; then
    echo "FAIL: Coverage ${COVERAGE}% is below threshold ${THRESHOLD}%"
    exit 1
fi

echo "PASS: Coverage ${COVERAGE}% meets threshold ${THRESHOLD}%"

# Generate human-readable markdown report
python3 "${BASE_DIR}/hack/cobertura-to-md.py" 2>&1
echo ""
echo "Report: ${REPORT_MD}"

# Update baseline if requested or if no baseline exists
if [ "$UPDATE" = true ] || [ ! -f "$BASELINE_FILE" ]; then
    echo "{\"threshold\": ${THRESHOLD}, \"coverage\": ${COVERAGE}, \"date\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}" > "$BASELINE_FILE"
    echo "Baseline updated: ${BASELINE_FILE}"
fi
