# Aether Workspace Justfile

default:
	@just --list

# ==========================================
# Testing
# ==========================================

# Run all tests
test:
	cargo test -- --nocapture

# Run tests with nextest (faster, parallel execution)
nt:
	cargo nextest run --workspace --all-targets

# ==========================================
# Pre-commit hooks
# ==========================================

# Install pre-commit hooks
setup-hooks:
	pre-commit install

# Run pre-commit on all files
qa:
	pre-commit run --all-files

# ==========================================
# Coverage
# ==========================================

# Run coverage check (tarpaulin + markdown report + threshold)
coverage:
	hack/cargo-coverage.sh

# Update coverage baseline
coverage-update:
	hack/cargo-coverage.sh --update-baseline
