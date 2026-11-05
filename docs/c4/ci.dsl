// C3/C4 for ci.
// Source: .github/workflows/ci.yml and Cargo.toml [package].
// The container sits in dxl, not in the external ci system: workspace.dsl sets `scope softwaresystem`,
// which rejects containers in a second software system. The external ci system runs it.

!element dxl {
    ci_workflows = container "CI workflows" "Workflow files in this repo that GitHub Actions runs to gate every PR and every push to main." "GitHub Actions YAML, .github/workflows/ci.yml"
}

ci -> dxl.ci_workflows "Runs the jobs of" "GitHub Actions runners"

!element dxl.ci_workflows {
    // C3: components (job families of ci.yml)
    ci_policy = component "ci: Workflow policy" "Triggers on every PR and push to main with no path filter; read-only contents permission, no secrets; a newer push cancels the older PR run; RUSTFLAGS=-D warnings for every job." "GitHub Actions workflow settings"
    ci_lint_test = component "ci: Lint and test" "fmt check, clippy (default, mock, all features) and workspace tests on Linux; std build and tests on Windows and macOS." "cargo fmt / clippy / test"
    ci_feature_sets = component "ci: Feature sets" "Derives every feature set from Cargo.toml [features] (no workflow edit when a feature is added), builds each alone on host and thumbv7em, tests each heap-free feature with std, and aggregates the legs into one required check." "cargo metadata + jq, job matrix"
    ci_no_std = component "ci: no_std build" "Clippy and builds of the library for thumbv7em-none-eabihf: no features, alloc alone, all heap-free features, all non-std features." "cargo build --target thumbv7em-none-eabihf"
    ci_coverage = component "ci: Coverage gate" "Workspace line coverage, generated models excluded; publishes the figure and the 70 % threshold to the run summary, then fails under 70 %." "cargo llvm-cov"
    ci_docs = component "ci: Docs and compile-fail" "rustdoc with -D warnings (all features, no default features) and compile_fail doc examples checked for their exact error code on a pinned toolchain." "cargo doc / cargo test --doc, RUSTC_BOOTSTRAP"
    ci_codegen = component "ci: Codegen diff check" "Reruns cargo xtask codegen and fails if src/models.rs or src/models/ differ from the committed files (FR-7.3)." "cargo xtask codegen + git diff"
    ci_tables = component "ci: Table check" "Runs the control table checker and its unit tests on every PR (FR-7.2, FR-7.6, NFR-9)." "python3"

    ci_policy -> ci_lint_test "Starts on every PR and push to main"
    ci_policy -> ci_feature_sets "Starts on every PR and push to main"
    ci_policy -> ci_coverage "Starts on every PR and push to main"
    ci_policy -> ci_docs "Starts on every PR and push to main; pins COMPILE_FAIL_TOOLCHAIN for"
    ci_policy -> ci_codegen "Starts on every PR and push to main"
    ci_policy -> ci_tables "Starts on every PR and push to main"
    ci_feature_sets -> ci_no_std "Passes heap_free and no_std feature lists to" "job outputs"
    ci_feature_sets -> ci_docs "Passes no_std feature list to" "job outputs"

    ci_lint_test -> dxl.lib "Formats, lints and tests" "cargo fmt / clippy / test"
    ci_lint_test -> dxl.xtask "Lints and tests (workspace member)" "cargo clippy / test --workspace"
    ci_feature_sets -> dxl.lib "Reads [features] of, then builds and tests each set of" "cargo metadata, cargo build / test --lib"
    ci_no_std -> dxl.lib "Builds without std, heap-free for" "thumbv7em-none-eabihf"
    ci_coverage -> dxl.lib "Measures line coverage of" "cargo llvm-cov"
    ci_coverage -> dxl.xtask "Measures line coverage of" "cargo llvm-cov --workspace"
    ci_docs -> dxl.lib "Builds docs and runs compile-fail doc examples of" "rustdoc"
    ci_codegen -> dxl.xtask "Reruns" "cargo xtask codegen"
    ci_codegen -> dxl.lib.models "Fails on any diff in" "git diff --exit-code"
    ci_codegen -> dxl.lib.kind "Fails on any diff in" "git diff --exit-code"
    ci_tables -> dxl.tables "Validates" "python3 control_tables/check_tables.py"

    // C4: code elements
    group "ci: Workflow policy" {
        ci_wf = component ".github/workflows/ci.yml" "Workflow CI: on pull_request and push to main; permissions contents: read; concurrency group per PR or ref, cancel-in-progress on PRs; env CARGO_TERM_COLOR, RUSTFLAGS=-D warnings, COMPILE_FAIL_TOOLCHAIN=1.97.1; every job checks out with persist-credentials: false and has a 20 min timeout." "GitHub Actions workflow" {
            tags "Code"
        }
        ci_cargo_package = component "Cargo.toml [package]" "name dxl_rs_v2, version 0.1.0, edition 2024; no rust-version (no MSRV, FR-010) and no [lints] table: missing_docs and mod_module_files are crate attributes in src/lib.rs, made fatal by RUSTFLAGS." "TOML" {
            tags "Code"
        }
    }

    group "ci: Lint and test" {
        ci_job_check = component ".github/workflows/ci.yml#check" "Job 'fmt, clippy, test (Linux)': cargo fmt --check; clippy --workspace --all-targets with default, mock and all features; cargo test --workspace; cargo test --features mock --doc; cargo test --workspace --all-features." "GitHub Actions job, ubuntu-latest, stable" {
            tags "Code"
        }
        ci_job_os = component ".github/workflows/ci.yml#os" "Job 'std build and tests (Windows / macOS)': cargo build and cargo test -p dxl_rs_v2 with default features (std, serialport)." "GitHub Actions job, matrix windows-latest + macos-latest" {
            tags "Code"
        }
    }

    group "ci: Feature sets" {
        ci_job_feature_sets = component ".github/workflows/ci.yml#feature-sets" "Job 'feature sets: list': jq over cargo metadata computes each feature's closure and emits outputs matrix (host: none, each alone, defaults, all; thumbv7em: each non-std feature alone), tests (std, std + each heap-free feature), heap_free and no_std lists; asserts the lists are non-empty." "GitHub Actions job, cargo metadata + jq" {
            tags "Code"
        }
        ci_job_feature_set = component ".github/workflows/ci.yml#feature-set" "Job 'feature set (<where>): <name>': cargo build --lib per matrix leg; installs libudev only for legs that need std." "GitHub Actions job, fromJSON matrix, fail-fast off" {
            tags "Code"
        }
        ci_job_feature_test = component ".github/workflows/ci.yml#feature-test" "Job 'feature tests (host): <name>': cargo test --lib per tests leg." "GitHub Actions job, fromJSON matrix, fail-fast off" {
            tags "Code"
        }
        ci_job_features = component ".github/workflows/ci.yml#features" "Job 'feature sets' (if: always()): fails unless the list job and every build and test leg succeeded; the one stable check name for the matrix." "GitHub Actions job, bash" {
            tags "Code"
        }
    }

    group "ci: no_std build" {
        ci_job_no_std = component ".github/workflows/ci.yml#no-std" "Job 'no_std build (thumbv7em-none-eabihf)': clippy with the heap-free set, then cargo build --lib with no features, alloc alone, heap-free set and no_std set." "GitHub Actions job, needs feature-sets" {
            tags "Code"
        }
    }

    group "ci: Coverage gate" {
        ci_job_coverage = component ".github/workflows/ci.yml#coverage" "Job 'coverage (>= 70 % lines)': cargo llvm-cov --workspace --no-report; JSON summary (src/models.rs and src/models/ excluded) written as a table to GITHUB_STEP_SUMMARY plus a notice; gate step --fail-under-lines 70." "GitHub Actions job, cargo-llvm-cov + jq" {
            tags "Code"
        }
    }

    group "ci: Docs and compile-fail" {
        ci_job_docs = component ".github/workflows/ci.yml#docs" "Job 'docs: complete, no warnings': cargo doc --no-deps with --all-features and --no-default-features under RUSTDOCFLAGS=-D warnings." "GitHub Actions job, rustdoc" {
            tags "Code"
        }
        ci_job_compile_fail = component ".github/workflows/ci.yml#compile-fail" "Job 'doc examples: compile-fail error codes': doc tests on the pinned COMPILE_FAIL_TOOLCHAIN with RUSTC_BOOTSTRAP=1 so compile_fail,E#### codes are enforced; all features, defaults, and the no_std set." "GitHub Actions job, pinned toolchain 1.97.1" {
            tags "Code"
        }
    }

    group "ci: Codegen diff check" {
        ci_job_codegen = component ".github/workflows/ci.yml#codegen" "Job 'generated models up to date': cargo xtask codegen, git add --intent-to-add so new files count, git diff --exit-code on src/models.rs and src/models/." "GitHub Actions job, rustfmt" {
            tags "Code"
        }
    }

    group "ci: Table check" {
        ci_job_control_tables = component ".github/workflows/ci.yml#control-tables" "Job 'control tables: schema check': python3 control_tables/check_tables.py, then unittest discover on control_tables/test_*.py; no Rust toolchain." "GitHub Actions job, python3" {
            tags "Code"
        }
    }

    ci_wf -> ci_job_check "Defines, with RUSTFLAGS=-D warnings"
    ci_wf -> ci_job_compile_fail "Pins COMPILE_FAIL_TOOLCHAIN for"
    ci_cargo_package -> ci_job_feature_sets "Is read by" "cargo metadata"
    ci_job_feature_sets -> ci_job_feature_set "Feeds the build matrix to" "outputs.matrix"
    ci_job_feature_sets -> ci_job_feature_test "Feeds the test matrix to" "outputs.tests"
    ci_job_feature_sets -> ci_job_no_std "Feeds heap_free and no_std lists to" "outputs.heap_free, outputs.no_std"
    ci_job_feature_sets -> ci_job_compile_fail "Feeds the no_std list to" "outputs.no_std"
    ci_job_features -> ci_job_feature_sets "Requires success of" "needs"
    ci_job_features -> ci_job_feature_set "Requires success of every leg of" "needs"
    ci_job_features -> ci_job_feature_test "Requires success of every leg of" "needs"

    ci_job_check -> dxl.lib.transport "Runs MockTransport doc tests of" "cargo test --features mock --doc"
    ci_job_os -> dxl.lib.transport "Builds the serialport std adapter of" "cargo build, Windows / macOS"
    ci_job_no_std -> dxl.lib "Builds heap-free and without std" "thumbv7em-none-eabihf"
    ci_job_compile_fail -> dxl.lib.multi "Checks that P2-only ops are absent on P1 (E0599) in" "compile_fail doc tests"
    ci_job_compile_fail -> dxl.lib.errors "Checks that #[non_exhaustive] forces a _ arm (E0004) in" "compile_fail doc tests"
    ci_job_compile_fail -> dxl.lib.handle "Checks that raw access is absent without raw-registers (E0599) and handles cannot outlive the bus (E0597) in" "compile_fail doc tests"
    ci_job_codegen -> dxl.xtask "Runs" "cargo xtask codegen"
    ci_job_control_tables -> dxl.tables "Checks every table and dynamixel.ron in" "python3"
}
