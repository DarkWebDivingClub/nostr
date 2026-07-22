#!/usr/bin/env just --justfile

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

[private]
default:
    @just --list

# Execute the pre-commit checks
precommit: fmt check-crates check-docs

# Execute continuous integration (CI) checks
ci: check-fmt check-crates check-docs

# Format the entire Rust code
fmt:
    @bash contrib/scripts/fmt.sh

# Check if the Rust code is formatted
[private]
check-fmt:
    @bash contrib/scripts/fmt.sh check

# Check all the crates
[private]
check-crates:
    @bash contrib/scripts/check-crates.sh

# Check Rust docs
[private]
check-docs:
    @bash contrib/scripts/check-docs.sh

# Release rust crates
[confirm]
release:
    cargo +stable publish --workspace

# Run code coverage using cargo-llvm-cov.
#
# Requires:
# - cargo-llvm-cov (install via: cargo install cargo-llvm-cov)
# - llvm-tools-preview component (install via: rustup component add llvm-tools-preview)
coverage package='none':
    cargo llvm-cov clean --workspace
    cargo llvm-cov --html {{ if package == 'none' { '--workspace' } else { '--package ' + package } }}
    @echo
    @echo 'open {{ justfile_directory() }}/target/llvm-cov/html/index.html'

# Generate an HTML diff‑coverage report for the current branch (compared to master).
# Requires: cargo-llvm-cov, diff-cover, Git.
# Examples:
#   just diff-coverage                         # full workspace
#   just diff-coverage package=my-crate        # single crate
#   just diff-coverage features="feat1,feat2"  # with features
diff-coverage package='none' features='none':
    #!/usr/bin/env bash
    set -euo pipefail

    BRANCH=$(git rev-parse --abbrev-ref HEAD)
    if [[ "$BRANCH" == "master" ]]; then
        echo "Error: This recipe cannot run on the master branch." >&2
        exit 1
    fi

    cargo llvm-cov clean --workspace

    # Pick workspace or package
    if [[ "{{ package }}" == "none" ]]; then
        PKG="--workspace"
    else
        PKG="--package {{ package }}"
    fi

    # Only add --features if needed
    FEAT=""
    if [[ "{{ features }}" != "none" ]]; then
        FEAT="--features {{ features }}"
    fi

    cargo llvm-cov --cobertura $PKG $FEAT --output-path coverage.xml

    mkdir -p "{{ justfile_directory() }}/target/diff-cover/"
    diff-cover coverage.xml \
        --compare-branch master \
        --format "html:{{ justfile_directory() }}/target/diff-cover/report.html"

    rm coverage.xml
    echo "Report: file://{{ justfile_directory() }}/target/diff-cover/report.html"

# Run benches (unstable)
bench:
    RUSTFLAGS='--cfg=bench' cargo +nightly bench
