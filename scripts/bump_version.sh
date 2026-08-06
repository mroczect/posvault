#!/usr/bin/env bash
set -euo pipefail

# Version manager for posvault workspace
# Ensures all crates share the same version.

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

usage() {
    cat <<'EOF'
Usage: $0 [--dry-run] <version|major|minor|patch>
  --dry-run   Show what would be changed without writing files.
EOF
    exit 1
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dry-run) DRY_RUN=true ;;
        -h|--help) usage ;;
        *) break ;;
    esac
    shift
done

if [[ $# -ne 1 ]]; then
    usage
fi

INPUT="$1"

CURRENT=""
for crate in "${CRATES[@]}"; do
    CARGOTOML="$WORKSPACE_ROOT/$crate/Cargo.toml"
    if [[ ! -f "$CARGOTOML" ]]; then
        echo "ERROR: $CARGOTOML not found" >&2
        exit 1
    fi
    CURRENT=$(grep -E '^version\s*=\s*"[0-9]+\.[0-9]+\.[0-9]+"' "$CARGOTOML" | head -1 | sed -E 's/.*"([^"]+)".*/\1/')
    if [[ -n "$CURRENT" ]]; then
        break
    fi
done

if [[ -z "$CURRENT" ]]; then
    echo "ERROR: Could not determine current version from any crate." >&2
    exit 1
fi

if ! echo "$CURRENT" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "ERROR: Invalid current version format: $CURRENT" >&2
    exit 1
fi

case "$INPUT" in
    major)   NEW=$(echo "$CURRENT" | awk -F. '{print $1+1".0.0"}') ;;
    minor)   NEW=$(echo "$CURRENT" | awk -F. '{print $1"."$2+1".0"}') ;;
    patch)   NEW=$(echo "$CURRENT" | awk -F. '{print $1"."$2"."$3+1}') ;;
    *)       NEW="$INPUT" ;;
esac

if ! echo "$NEW" | grep -Eq '^[0-9]+\.[0-9]+\.[0-9]+$'; then
    echo "ERROR: Invalid new version format: $NEW" >&2
    exit 1
fi

if [[ "$NEW" == "$CURRENT" ]]; then
    echo "Nothing to do: version is already $CURRENT."
    exit 0
fi

echo "Current version: $CURRENT"
echo "New version:      $NEW"

if ! git diff-index --quiet HEAD -- 2>/dev/null; then
    echo "WARNING: Working directory is not clean. Continue only if you know what you're doing."
fi

replace_version_in_file() {
    local file="$1"
    local old_ver="$2"
    local new_ver="$3"
    local dry="$4"

    if [[ "$dry" == "true" ]]; then
        echo "DRY RUN: would replace in $file: $old_ver -> $new_ver"
        return
    fi

    cp "$file" "$file.bak"

    if [[ "$OSTYPE" == "darwin"* ]]; then
        sed -i '' -E "s/^version\s*=\s*\"$old_ver\"/version = \"$new_ver\"/" "$file"
    else
        sed -i -E "s/^version\s*=\s*\"$old_ver\"/version = \"$new_ver\"/" "$file"
    fi

    for dep in "${CRATES[@]}"; do
        if [[ "$OSTYPE" == "darwin"* ]]; then
            sed -i '' -E "s/(${dep}\s*=\s*\{[^}]*version\s*=\s*)\"$old_ver\"/\1\"$new_ver\"/" "$file"
            sed -i '' -E "s/(${dep}\s*=\s*)\"$old_ver\"/\1\"$new_ver\"/" "$file"
        else
            sed -i -E "s/(${dep}\s*=\s*\{[^}]*version\s*=\s*)\"$old_ver\"/\1\"$new_ver\"/" "$file"
            sed -i -E "s/(${dep}\s*=\s*)\"$old_ver\"/\1\"$new_ver\"/" "$file"
        fi
    done
}

for crate in "${CRATES[@]}"; do
    TOML="$WORKSPACE_ROOT/$crate/Cargo.toml"
    echo "Updating $crate/Cargo.toml..."
    replace_version_in_file "$TOML" "$CURRENT" "$NEW" "$DRY_RUN"
done

if ! $DRY_RUN; then
    echo "Refreshing Cargo.lock..."
    cargo update --workspace 2>&1 || echo "WARNING: cargo update failed, please run manually."
fi

echo ""
if $DRY_RUN; then
    echo "Dry run complete. Run without --dry-run to apply changes."
else
    echo "All crates updated to version $NEW. Backup files with .bak extension created."
    echo "Please review changes, then remove backup files and commit."
fi
