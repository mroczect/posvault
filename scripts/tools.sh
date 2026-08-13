#!/usr/bin/env bash
set -euo pipefail

WORKSPACE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATES=(
	"posvault"
	"posvault_auth"
	"posvault_crypto"
	"posvault_handler"
	"posvault_query"
	"posvault_sign"
	"posvault_store"
	"posvault_sync"
)
DRY_RUN=false

die() { echo -e "\033[1;31mERROR:\033[0m $*" >&2; exit 1; }
info() { echo -e "\033[1;34m→\033[0m $*"; }
success() { echo -e "\033[1;32m✓\033[0m $*"; }
warn() { echo -e "\033[1;33mWARNING:\033[0m $*" >&2; }

usage() {
    cat <<EOF
Usage: $0 [--dry-run] <command> [args...]

Commands:
  version <major|minor|patch|X.Y.Z>
                     Bump version across all crates.
  check              Run formatting, clippy, and tests.
  readme init        Generate missing README.md files.
  readme check       Check for missing README.md files.
  clean              Clean all build artifacts.
  doc                Open workspace documentation in browser.
  publish check      Check if all crates are ready for publishing.
  publish run        Publish crates to crates.io in dependency order.
  ci                 Run local CI checks (fmt, clippy, test).
  audit              Run security audit (requires cargo-audit).
  help               Show this help message.

Options:
  --dry-run          Don't actually modify anything, just show what would be done.
EOF
    exit 0
}

ensure_crates_exist() {
    for crate in "${CRATES[@]}"; do
        [[ -d "$WORKSPACE_ROOT/$crate" ]] || die "Folder $crate not found in $WORKSPACE_ROOT"
    done
}

check_clean_git() {
    if ! git diff-index --quiet HEAD -- 2>/dev/null; then
        warn "Working directory is not clean, proceed with caution."
    fi
}

cmd_version() {
    local input="$1"
    local current=""

    for crate in "${CRATES[@]}"; do
        local toml="$WORKSPACE_ROOT/$crate/Cargo.toml"
        [[ -f "$toml" ]] || continue
        current=$(grep -E '^version\s*=\s*"[0-9]+\.[0-9]+\.[0-9]+"' "$toml" | head -1 | sed -E 's/.*"([^"]+)".*/\1/')
        [[ -n "$current" ]] && break
    done

    [[ -z "$current" ]] && die "Failed to read current version from any crate."
    [[ ! "$current" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] && die "Current version format is weird: $current"

    local new
    case "$input" in
        major) new=$(echo "$current" | awk -F. '{print $1+1".0.0"}') ;;
        minor) new=$(echo "$current" | awk -F. '{print $1"."$2+1".0"}') ;;
        patch) new=$(echo "$current" | awk -F. '{print $1"."$2"."$3+1}') ;;
        *)     new="$input" ;;
    esac

    [[ ! "$new" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] && die "New version is invalid: $new"
    [[ "$new" == "$current" ]] && { info "Version is already $current, nothing to change."; return 0; }

    info "Bump version from $current to $new"

    if [[ "$DRY_RUN" == "true" ]]; then
        info "Dry run, no changes made."
        return 0
    fi

    check_clean_git

    for crate in "${CRATES[@]}"; do
        local toml="$WORKSPACE_ROOT/$crate/Cargo.toml"
        info "Processing $crate/Cargo.toml..."
        cp "$toml" "$toml.bak"

        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' -E "s/^version\s*=\s*\"$current\"/version = \"$new\"/" "$toml"
        else
            sed -i -E "s/^version\s*=\s*\"$current\"/version = \"$new\"/" "$toml"
        fi

        for dep in "${CRATES[@]}"; do
            if [[ "$OSTYPE" == "darwin"* ]]; then
                sed -i '' -E "s/(${dep}\s*=\s*\{[^}]*version\s*=\s*)\"$current\"/\1\"$new\"/" "$toml"
                sed -i '' -E "s/(${dep}\s*=\s*)\"$current\"/\1\"$new\"/" "$toml"
            else
                sed -i -E "s/(${dep}\s*=\s*\{[^}]*version\s*=\s*)\"$current\"/\1\"$new\"/" "$toml"
                sed -i -E "s/(${dep}\s*=\s*)\"$current\"/\1\"$new\"/" "$toml"
            fi
        done
    done

    success "All crates updated to version $new. Backup files (.bak) created. Don't forget to commit!"
}

cmd_check() {
    info "Checking formatting..."
    cargo fmt --all -- --check || die "Formatting is off, run 'cargo fmt --all' first."
    success "Formatting OK"

    info "Running clippy (strict)..."
    cargo clippy --all-targets --all-features -- -D warnings || die "Clippy found issues, please fix them."
    success "Clippy is happy"

    info "Running all tests..."
    RUST_BACKTRACE=1 cargo test --workspace -- --nocapture || die "Some tests failed."
    success "All tests passed"

    success "Workspace is ready for action."
}

cmd_readme() {
    ensure_crates_exist
    local sub="$1"
    case "$sub" in
        init)
            for crate in "${CRATES[@]}"; do
                local readme="$WORKSPACE_ROOT/$crate/README.md"
                if [[ -f "$readme" ]]; then
                    info "$crate/README.md already exists, skipping."
                else
                    if [[ "$DRY_RUN" == "true" ]]; then
                        info "DRY RUN: would create $readme"
                    else
                        echo "# $crate\n\nPart of the posvault workspace.\n\nSee [main README](../README.md) for full documentation." > "$readme"
                        success "Created $readme"
                    fi
                fi
            done
            ;;
        check)
            local missing=0
            for crate in "${CRATES[@]}"; do
                if [[ ! -f "$WORKSPACE_ROOT/$crate/README.md" ]]; then
                    warn "MISSING: $crate/README.md"
                    missing=$((missing+1))
                fi
            done
            if [[ $missing -gt 0 ]]; then
                die "$missing README file(s) missing. Run '$0 readme init' to create them."
            else
                success "All README files present."
            fi
            ;;
        *) die "Unknown subcommand for readme: use init or check" ;;
    esac
}

cmd_clean() {
    info "Cleaning target/ directories..."
    cargo clean
    success "Spotless."
}

cmd_doc() {
    info "Opening documentation in browser..."
    cargo doc --workspace --no-deps --open
}

cmd_publish() {
    local sub="${1:-check}"
    case "$sub" in
        check)
            info "Checking if all crates are ready to publish..."
            for crate in "${CRATES[@]}"; do
                echo "→ Packaging $crate"
                cargo package -p "$crate" --no-verify || die "Failed to package $crate"
            done
            success "All crates are ready to publish. Go for it!"
            ;;
        run)
            info "Publishing all crates in order..."
            check_clean_git
            local order=(
                "posvault_handler"
                "posvault_crypto"
                "posvault_store"
                "posvault_auth"
                "posvault_sign"
                "posvault_query"
                "posvault_sync"
                "posvault"
            )
            for crate in "${order[@]}"; do
                echo "→ Publishing $crate"
                cargo publish -p "$crate" || die "Failed to publish $crate"
                sleep 5
            done
            success "All crates published. Check crates.io!"
            ;;
        *) die "Unknown subcommand for publish: use check or run" ;;
    esac
}

cmd_ci() {
    info "Running local CI (fmt, clippy, test)..."
    cargo fmt --all -- --check || die "Formatting is off."
    cargo clippy --all-targets --all-features -- -D warnings || die "Clippy failed."
    RUST_BACKTRACE=1 cargo test --workspace -- --nocapture || die "Tests failed."
    success "Local CI passed. Safe to push."
}

cmd_audit() {
    if ! command -v cargo-audit &>/dev/null; then
        die "cargo-audit is not installed. Install with: cargo install cargo-audit"
    fi
    info "Running security audit..."
    cargo audit || die "Vulnerabilities found, check output above."
    success "No known vulnerabilities."
}

main() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --dry-run) DRY_RUN=true ;;
            -h|--help) usage ;;
            *) break ;;
        esac
        shift
    done

    cd "$WORKSPACE_ROOT"

    local cmd="${1:-help}"
    shift || true

    ensure_crates_exist

    case "$cmd" in
        version) cmd_version "$@" ;;
        check)   cmd_check "$@" ;;
        readme)  cmd_readme "$@" ;;
        clean)   cmd_clean ;;
        doc)     cmd_doc ;;
        publish) cmd_publish "$@" ;;
        ci)      cmd_ci ;;
        audit)   cmd_audit ;;
        help|*)  usage ;;
    esac
}

main "$@"
