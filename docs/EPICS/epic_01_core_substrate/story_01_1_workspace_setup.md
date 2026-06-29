# Story: Workspace Setup & Crate Layout

*   **Status:** Draft
*   **Story ID:** `STORY-01.1`
*   **Parent Epic:** [EPIC-01: Core API & Rust Substrate](file:///Users/casibbald/Workspace/remote/microscaler/Aether/docs/EPICS/epic_01_core_substrate/epic_01_core_substrate.md)
*   **Estimation:** [TBD]
*   **Owner:** [@username]

---

## 1. User Story Statement

```text
As a developer
I want set up the Rust Cargo workspace layout (Aggregator, Daemon, Auth, Fencing)
So that the team has a consistent, modular codebase to develop all components
```

## 2. Functional & Non-Functional Requirements

### A. Functional Requirements
*   **FR-1.1.1:** Root `Cargo.toml` must define workspace members for `aether-aggregator`, `aetherd`, `aether-auth`, and `aether-fence`.
*   **FR-1.1.2:** All crates must inherit shared workspace configurations, output directories, and dependencies.
*   **FR-1.1.3:** The root `Cargo.toml` must configure compiler and clippy lint groups in a `[workspace.lints]` section to enforce coding standards, forbid unsafe blocks, and limit cognitive complexity.
*   **FR-1.1.4:** Expose and configure profile options in `Cargo.toml` to support code coverage instrumentation (e.g., cargo-llvm-cov).
*   **FR-1.1.5:** Configure linting, formatting, and strict type checking (including Pydantic mypy plugins) for workspace Python scripts using a root `pyproject.toml` file.
*   **FR-1.1.6:** Establish a workspace-level `clippy.toml` to configure strict JSF-inspired static limits (cognitive complexity limit, maximum stack sizes, and argument counts).

### B. Non-Functional Requirements
*   **NFR-1.1.1:** Rust compilation targets must support both `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu` bare-metal architectures.
*   **NFR-1.1.2:** Core crates must build with zero external C dynamic dependencies to maintain local portability.
*   **NFR-1.1.3:** Enforce strict file length boundaries (e.g., maximum 500 lines per file) and cognitive complexity limits (maximum complexity of 15) to guarantee maximum code modularity.
*   **NFR-1.1.4:** Maintain a target minimum code coverage of **80%** on all shared library code.
*   **NFR-1.1.5:** Adhere to Joint Strike Fighter (JSF) Air Vehicle C++ Coding Standards:
    *   **No heap allocations in hot paths** (telemetry parsing, reverse-bid evaluations) using stack-allocated collections (e.g., `SmallVec`/`TinyVec`).
    *   **No panics/unwraps in runtime code** to ensure zero crash paths in worker loops.
    *   **No recursion** in scheduling and VM execution loops to guarantee bounded stack depth.

## 3. Technical Implementation Details

Provide detailed instructions for the engineer implementing this story. Map out the code structure, classes, traits, and configurations.

### A. Affected Codebases & Files
*   **Crate:** `Cargo Workspace (root)`
*   **Target Files:**
    *   [Cargo.toml](file:///Users/casibbald/Workspace/remote/microscaler/Aether/Cargo.toml)
    *   [clippy.toml](file:///Users/casibbald/Workspace/remote/microscaler/Aether/clippy.toml) (NEW)
    *   [pyproject.toml](file:///Users/casibbald/Workspace/remote/microscaler/Aether/pyproject.toml) (NEW)
    *   [crates/aether-aggregator/Cargo.toml](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-aggregator/Cargo.toml)
    *   [crates/aetherd/Cargo.toml](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aetherd/Cargo.toml)
    *   [crates/aether-auth/Cargo.toml](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-auth/Cargo.toml)
    *   [crates/aether-fence/Cargo.toml](file:///Users/casibbald/Workspace/remote/microscaler/Aether/crates/aether-fence/Cargo.toml)

### B. Detailed Design

#### 1. Root Cargo.toml Lint Configuration
The workspace root `Cargo.toml` must enforce unified quality checks across all sub-crates:
```toml
[workspace]
resolver = "2"
members = [
    "crates/aether-aggregator",
    "crates/aetherd",
    "crates/aether-auth",
    "crates/aether-fence",
]

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "warn"
unused_qualifications = "warn"

[workspace.lints.clippy]
cognitive_complexity = { level = "deny", limit = 15 }
type_complexity = "deny"
too_many_lines = { level = "deny" }  # Enforces maximum function/file modularity
pedantic = { level = "warn", priority = -1 }
```

#### 2. Root clippy.toml JSF Configuration
Create a workspace-level `clippy.toml` file to configure static analysis bounds aligned with JSF standards:
```toml
# clippy.toml - JSF AV Rules-inspired static bounds
cognitive-complexity-threshold = 15
stack-size-threshold = 512000
too-many-arguments-threshold = 8
```

#### 3. Enforcing "No Panics" Crate-level Lints
Every compiled Rust crate must declare lint constraints in its root module (e.g., `src/lib.rs` or `src/main.rs`) to ensure runtime safety:
```rust
// Enforce JSF rule of no panics in runtime paths
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
```
*Note:* Exceptions are allowed via local allowances (`#[allow(clippy::unwrap_used)]`) strictly confined to startup initialization paths (such as `fn main` parsing config files) or test files.

#### 4. Root pyproject.toml Quality Configuration
Configure Pydantic linting, ruff, and strict mypy constraints for Python scripts:
```toml
[tool.ruff]
line-length = 100
target-version = "py310"
select = ["E", "F", "I", "N", "UP", "B", "A", "C4", "SIM", "RUF"]

[tool.mypy]
python_version = "3.10"
strict = true
plugins = ["pydantic.mypy"]

[tool.pydantic-mypy]
init_forbid_extra = true
init_typed = true
warn_required_dynamic_aliases = true
```

## 4. Acceptance Criteria

Use Gherkin format (`Given/When/Then`) or precise checklist items to specify testable criteria:

*   **Criteria 1:**
    *   **Given** a clean workspace directory
    *   **When** running `cargo build --workspace`
    *   **Then** all crates must compile successfully with zero errors or warnings.
*   **Criteria 2:**
    *   **Given** a source file containing an `unsafe` block or exceeding cognitive complexity 15
    *   **When** running `cargo clippy --workspace --all-targets`
    *   **Then** the compiler must reject compilation and report clippy lint errors.
*   **Criteria 3:**
    *   **Given** a Python script violating Pydantic or Ruff constraints
    *   **When** running `ruff check .` or `mypy .`
    *   **Then** the validation check must exit with error codes.
*   **Criteria 4:**
    *   **Given** a runtime source file containing a `.unwrap()`, `.expect()`, or `panic!` statement
    *   **When** running `cargo clippy --workspace --all-targets`
    *   **Then** the compiler must reject compilation and flag the panic path as a denial error.

## 5. Verification Plan

Detail how to test and verify the implementation:

### A. Automated Tests
*   **Rust Compilation:** Run `cargo build --workspace`
*   **Rust Lints & Complexity:** Run `cargo clippy --workspace --all-targets -- -D warnings`
*   **Code Coverage Check:** Run `cargo llvm-cov --workspace --all-targets`
*   **Python Lints & Types:** Run `ruff check .` and `mypy .`

### B. Manual Verification
*   **Step 1:** Intentionally add an unsafe block (`unsafe {}`) to `crates/aether-auth/src/lib.rs` and verify that `cargo clippy` fails.
*   **Step 2:** Intentionally add a Python script violating Pydantic type signatures to `docs/` and verify that `mypy` catches the error.
*   **Step 3:** Intentionally write a runtime `.unwrap()` statement in `crates/aether-aggregator/src/registry.rs` and verify that `cargo clippy` fails, validating our JSF "No Panics" rule compliance.
