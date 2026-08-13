#!/usr/bin/env bash
set -euo pipefail

###############################################################################
# Release preparation script for posvault workspace.
# Runs tests & clippy, then creates and pushes a git tag to trigger CI.
###############################################################################

readonly LOG_FILE="release_$(date +%Y%m%d_%H%M%S).log"

readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[0;33m'
readonly NC='\033[0m'

log() {
    local level="$1"; shift
    local msg="$*"
    local timestamp
    timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    case "$level" in
        INFO)  printf "[%s] ${GREEN}INFO${NC}  %s\n" "$timestamp" "$msg" | tee -a "$LOG_FILE" ;;
        WARN)  printf "[%s] ${YELLOW}WARN${NC}  %s\n" "$timestamp" "$msg" | tee -a "$LOG_FILE" ;;
        ERROR) printf "[%s] ${RED}ERROR${NC} %s\n" "$timestamp" "$msg" | tee -a "$LOG_FILE" >&2 ;;
        *)     printf "[%s] %s\n" "$timestamp" "$msg" | tee -a "$LOG_FILE" ;;
    esac
}

error_exit() {
    log ERROR "$1"
    exit 1
}

check_prerequisites() {
    for cmd in cargo jq git gh; do
        if ! command -v "$cmd" &>/dev/null; then
            error_exit "$cmd not found. Please install it and ensure it is in PATH."
        fi
    done
    if ! gh auth status &>/dev/null; then
        error_exit "gh CLI not authenticated. Run 'gh auth login' first."
    fi
}

ensure_clean_workspace() {
    if ! git diff-index --quiet HEAD --; then
        error_exit "There are uncommitted changes. Commit or stash them first."
    fi
    local branch
    branch=$(git rev-parse --abbrev-ref HEAD)
    if [ "$branch" != "master" ] && [ "$branch" != "main" ]; then
        log WARN "You are on branch '$branch', not master/main."
        read -r -p "Continue? (y/n) " confirm
        if [ "$confirm" != "y" ]; then
            log INFO "Aborted by user."
            exit 0
        fi
    fi
    git fetch origin
    local local_commit remote_commit
    local_commit=$(git rev-parse HEAD)
    remote_commit=$(git rev-parse "origin/$branch")
    if [ "$local_commit" != "$remote_commit" ]; then
        log WARN "Local branch is not up to date with origin/$branch. Pull first."
        read -r -p "Continue anyway? (y/n) " confirm
        if [ "$confirm" != "y" ]; then
            log INFO "Aborted by user."
            exit 0
        fi
    fi
}

get_publishable_crates() {
    cargo metadata --no-deps --format-version 1 2>/dev/null \
        | jq -r '
            .packages[]
            | select(
                .publish == null
                or .publish == true
                or (.publish | type == "array" and length > 0)
              )
            | "\(.name) \(.version) \(.manifest_path)"
          '
}

prepare_crate() {
    local crate_name="$1"
    local version="$2"
    local manifest_path="$3"
    local manifest_dir
    manifest_dir=$(dirname "$manifest_path")

    log INFO "-----------------------------------------------"
    log INFO "Preparing release for: $crate_name v$version"
    cd "$manifest_dir" || error_exit "Failed to enter $manifest_dir"

    log INFO "Running tests..."
    if ! cargo test -p "$crate_name" --all-targets; then
        cd - > /dev/null
        error_exit "Tests failed for $crate_name."
    fi

    log INFO "Running clippy..."
    if ! cargo clippy -p "$crate_name" --all-targets -- -D warnings; then
        cd - > /dev/null
        error_exit "Clippy failed for $crate_name."
    fi

    local tag="${crate_name}@${version}"
    log INFO "Creating tag $tag..."
    if git rev-parse "$tag" >/dev/null 2>&1; then
        log WARN "Tag $tag already exists, skipping."
    else
        git tag -a "$tag" -m "Release $crate_name v$version"
        git push origin "$tag"
        log INFO "Tag $tag pushed. CI will now publish to crates.io."
    fi

    cd - > /dev/null
    log INFO "Done."
}

main() {
    TARGET_CRATE=""
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -h|--help)
                echo "Usage: $0 [crate_name]"
                exit 0
                ;;
            *)
                TARGET_CRATE="$1"
                shift
                ;;
        esac
    done

    log INFO "Release preparation started for posvault workspace."
    check_prerequisites
    ensure_clean_workspace

    local crates
    crates=$(get_publishable_crates)
    if [ -z "$crates" ]; then
        error_exit "No publishable crates found."
    fi

    if [ -n "$TARGET_CRATE" ]; then
        local found=0
        while IFS= read -r line; do
            name=$(echo "$line" | awk '{print $1}')
            ver=$(echo "$line" | awk '{print $2}')
            manifest=$(echo "$line" | awk '{print $3}')
            if [ "$name" = "$TARGET_CRATE" ]; then
                found=1
                prepare_crate "$name" "$ver" "$manifest"
                break
            fi
        done <<< "$crates"
        if [ "$found" -eq 0 ]; then
            error_exit "Crate '$TARGET_CRATE' not found."
        fi
    else
        while IFS= read -r line; do
            name=$(echo "$line" | awk '{print $1}')
            ver=$(echo "$line" | awk '{print $2}')
            manifest=$(echo "$line" | awk '{print $3}')
            prepare_crate "$name" "$ver" "$manifest"
        done <<< "$crates"
    fi

    log INFO "All done. Check the CI pipeline for publication status."
}

main "$@"
