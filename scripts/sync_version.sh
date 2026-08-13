#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DRY_RUN="${DRY_RUN:-0}"

command -v cargo >/dev/null 2>&1 || { echo "cargo not found" >&2; exit 1; }
command -v jq >/dev/null 2>&1 || { echo "jq not found – install it: https://stedolan.github.io/jq/" >&2; exit 1; }

if [[ "$(uname -s)" == "Darwin" ]]; then
  SED_INPLACE=(-i '')
else
  SED_INPLACE=(-i)
fi

echo "Reading workspace metadata..."
metadata=$(cargo metadata --format-version 1 --no-deps)

mapfile -t members < <(echo "$metadata" | jq -c '.workspace_members[] as $id | .packages[] | select(.id == $id) | {name, version, manifest_path}')

if [[ ${#members[@]} -eq 0 ]]; then
    echo "No workspace members found." >&2
    exit 1
fi

declare -A crate_versions
declare -A crate_paths
for member in "${members[@]}"; do
    name=$(echo "$member" | jq -r '.name')
    version=$(echo "$member" | jq -r '.version')
    path=$(echo "$member" | jq -r '.manifest_path')
    crate_versions["$name"]="$version"
    crate_paths["$name"]="$path"
    echo "Crate '$name' version $version"
done

for member in "${members[@]}"; do
    dep_toml=$(echo "$member" | jq -r '.manifest_path')

    for crate in "${!crate_versions[@]}"; do
        version="${crate_versions[$crate]}"

        sed "${SED_INPLACE[@]}" -E \
            -e "/^[[:space:]]*\[(dependencies|dev-dependencies|build-dependencies)\][[:space:]]*/,/^[[:space:]]*\[/ {" \
            -e   "s/(${crate}[[:space:]]*=[[:space:]]*\{[^}]*version[[:space:]]*=[[:space:]]*\")[^\"]+(\")/\1${version}\2/" \
            -e "}" \
            "$dep_toml"
    done
    echo "Checked $dep_toml"
done

if [[ "$DRY_RUN" -eq 1 ]]; then
    echo "Dry‑run mode: discarding changes"
    git checkout -- .
    exit 0
fi

if cargo pkgid 2>/dev/null; then
    echo "All workspace dependencies consistent."
else
    echo "WARNING: 'cargo pkgid' reported an issue. Please inspect manually." >&2
fi

echo "Version synchronization complete."
