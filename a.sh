#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# PosVault Roadmap Issue Generator - Comprehensive Edition (Fixed)
# Requires: gh CLI (authenticated), jq
# Usage:
#   1. Set OWNER and REPO below (or export them before running)
#   2. Run: ./create_posvault_issues.sh
# =============================================================================

OWNER="${OWNER:-mroczect}"
REPO="${REPO:-posvault}"

# -----------------------------------------------------------------------------
# Helper: create milestone if it does not exist
# -----------------------------------------------------------------------------
create_milestone() {
  local title="$1"
  local description="$2"

  local existing
  existing=$(gh api --paginate "repos/$OWNER/$REPO/milestones?state=all" 2>/dev/null \
    | jq -r --arg t "$title" '.[] | select(.title == $t) | .number' \
    | head -n1)

  if [[ -n "$existing" ]]; then
    echo "Milestone already exists: $title (#$existing)" >&2
    echo "$existing"
  else
    local number
    number=$(gh api --method POST "repos/$OWNER/$REPO/milestones" \
      -f title="$title" \
      -f description="$description" \
      --jq '.number')
    echo "Created milestone: $title (#$number)" >&2
    echo "$number"
  fi
}

# -----------------------------------------------------------------------------
# Helper: create label if it does not exist
# -----------------------------------------------------------------------------
create_label() {
  local name="$1"
  local color="$2"
  local description="$3"

  local existing
  existing=$(gh api --paginate "repos/$OWNER/$REPO/labels?per_page=100" 2>/dev/null \
    | jq -r --arg n "$name" '.[] | select(.name == $n) | .name' \
    | head -n1)

  if [[ -n "$existing" ]]; then
    echo "Label already exists: $name" >&2
  else
    gh label create "$name" --color "$color" --description "$description" --repo "$OWNER/$REPO"
    echo "Created label: $name" >&2
  fi
}

# -----------------------------------------------------------------------------
# Helper: create issue linked to milestone and labels
# -----------------------------------------------------------------------------
create_issue() {
  local title="$1"
  local body="$2"
  local milestone="$3"
  shift 3
  local labels=("$@")

  local label_args=()
  for label in "${labels[@]}"; do
    label_args+=(--label "$label")
  done

  gh issue create \
    --repo "$OWNER/$REPO" \
    --title "$title" \
    --body "$body" \
    --milestone "$milestone" \
    "${label_args[@]}"
}

# =============================================================================
# 1. CREATE MILESTONES
# =============================================================================
M1=$(create_milestone "Phase 1 - Foundation (MVP)" \
  "Core library persistence, snapshot, session expiration, default encryption and signing.")
M2=$(create_milestone "Phase 2 - Security & Audit" \
  "RBAC, append-only audit, key management, anti-tamper, sync conflict resolution.")
M3=$(create_milestone "Phase 3 - POS Operational" \
  "Branch, stock management, daily sales, backup/restore, basic CLI.")

# =============================================================================
# 2. CREATE LABELS
# =============================================================================

# Priority labels
create_label "priority: critical" "b60205" "Critical priority"
create_label "priority: high"     "d93f0b" "High priority"
create_label "priority: medium"   "fbca04" "Medium priority"
create_label "priority: low"      "0e8a16" "Low priority"

# Phase labels
create_label "phase1"             "0052cc" "Phase 1 - Foundation"
create_label "phase2"             "006b75" "Phase 2 - Security & Audit"
create_label "phase3"             "d4c5f9" "Phase 3 - POS Operational"

# Module labels
create_label "module: posvault"           "1d76db" "Umbrella facade"
create_label "module: posvault_handler"   "2374ab" "Core types and traits"
create_label "module: posvault_auth"      "a8322a" "Authentication and RBAC"
create_label "module: posvault_crypto"    "6432a8" "Encryption/decryption"
create_label "module: posvault_query"     "7957d5" "Query engine"
create_label "module: posvault_sign"      "c5b3f9" "Signing"
create_label "module: posvault_store"     "e76f51" "Storage"
create_label "module: posvault_sync"      "2ec4b6" "Sync and branch"

# Type labels
create_label "type: feature" "0e8a16" "New feature"
create_label "type: test"    "fbca04" "Testing"
create_label "type: docs"    "c5def5" "Documentation"
create_label "type: chore"   "cccccc" "Maintenance"

# =============================================================================
# 3. CREATE ISSUES
# =============================================================================

# -----------------------------------------------------------------------------
# Phase 1 - Foundation
# -----------------------------------------------------------------------------

# Disk persistence
create_issue "Design DiskStore trait and API" \
"$(cat <<'EOF'
**Goal:** Define the traits and error types for disk-based object storage.

**Module:** `posvault_store`
**Type:** feature

**Deliverables:**
- `DiskObjectStore` trait or concrete struct API
- `DiskRefStore` API
- Error variants for disk I/O, corruption, and consistency.

**Acceptance Criteria:**
- API is documented and compiles.
- No implementation is required in this task.

**Priority:** Critical
EOF
)" "$M1" "priority: critical" "phase1" "module: posvault_store" "type: feature"

create_issue "Implement DiskObjectStore with file-backed storage" \
"$(cat <<'EOF'
**Goal:** Store objects on disk using their hash as filename or directory path.

**Module:** `posvault_store`
**Type:** feature

**Deliverables:**
- `DiskObjectStore` struct
- Implement `ObjectStore` trait
- Atomic write, read, delete, exists
- Directory layout: `<base>/objects/xx/yyyy...`

**Acceptance Criteria:**
- Objects survive process restart.
- `put` then `get` returns exact bytes.
- `exists` and `delete` behave correctly.

**Priority:** Critical
EOF
)" "$M1" "priority: critical" "phase1" "module: posvault_store" "type: feature"

create_issue "Implement DiskRefStore with file-backed refs" \
"$(cat <<'EOF'
**Goal:** Store named references (branches, tags, HEAD) on disk.

**Module:** `posvault_store`
**Type:** feature

**Deliverables:**
- `DiskRefStore` struct
- Implement `RefStore` trait
- Use separate files for each ref under `<base>/refs/`

**Acceptance Criteria:**
- Refs survive restart.
- `set_ref`, `get_ref`, `delete_ref` and `list_refs` work.

**Priority:** Critical
EOF
)" "$M1" "priority: critical" "phase1" "module: posvault_store" "type: feature"

create_issue "Integrate DiskObjectStore and DiskRefStore into FileStore" \
"$(cat <<'EOF'
**Goal:** Replace in-memory stores with disk stores in `FileStore`.

**Module:** `posvault_store`
**Type:** feature

**Deliverables:**
- `FileStore` uses `DiskObjectStore` and `DiskRefStore`
- Keep existing public API of `FileStore` unchanged
- Add open/close semantics with path.

**Acceptance Criteria:**
- `PosVault::open(path)` persists data after restart.
- All existing store tests pass with disk backend.

**Priority:** Critical
EOF
)" "$M1" "priority: critical" "phase1" "module: posvault_store" "type: feature"

# Automatic snapshots
create_issue "Design snapshot trigger policy" \
"$(cat <<'EOF'
**Goal:** Determine when snapshots should be taken automatically.

**Module:** `posvault_query`, `posvault_store`
**Type:** feature

**Deliverables:**
- Define threshold `SNAPSHOT_INTERVAL`
- Define trigger condition: number of events since last snapshot.

**Acceptance Criteria:**
- Policy is documented and constants are defined.

**Priority:** Critical
EOF
)" "$M1" "priority: critical" "phase1" "module: posvault_query" "type: feature"

create_issue "Implement automatic snapshot creation in QueryEngine" \
"$(cat <<'EOF'
**Goal:** Create snapshots without manual invocation.

**Module:** `posvault_query`
**Type:** feature

**Deliverables:**
- After each `get_events_since`, check threshold
- If exceeded, call `save_snapshot`
- Update cache.

**Acceptance Criteria:**
- Snapshots are created automatically after enough events.
- Subsequent queries use the latest snapshot.

**Priority:** Critical
EOF
)" "$M1" "priority: critical" "phase1" "module: posvault_query" "type: feature"

# Session expiration
create_issue "Enforce session validation on every guarded operation" \
"$(cat <<'EOF'
**Goal:** Prevent expired sessions from performing any action.

**Module:** `posvault_auth`
**Type:** feature

**Deliverables:**
- `require_role` checks `session.is_expired()`
- `PosVault` checks session before `transact`, `journal`, `query`, `sync`

**Acceptance Criteria:**
- Expired session is rejected with `Auth` error.
- Valid session passes.

**Priority:** Critical
EOF
)" "$M1" "priority: critical" "phase1" "module: posvault_auth" "type: feature"

create_issue "Add session refresh and absolute TTL" \
"$(cat <<'EOF'
**Goal:** Allow session extension and hard expiry.

**Module:** `posvault_auth`
**Type:** feature

**Deliverables:**
- `Session::refresh()` resets expiration but not absolute TTL
- Add `absolute_expiry` field if needed.

**Acceptance Criteria:**
- Refresh works correctly.
- Session cannot be extended beyond absolute limit.

**Priority:** Critical
EOF
)" "$M1" "priority: critical" "phase1" "module: posvault_auth" "type: feature"

# Default encryption
create_issue "Require recipients for event payload encryption" \
"$(cat <<'EOF'
**Goal:** Prevent plaintext events from entering storage.

**Module:** `posvault`, `posvault_crypto`
**Type:** feature

**Deliverables:**
- `PosVault::transact` returns error if recipients empty.
- Error variant `Encryption` with clear message.

**Acceptance Criteria:**
- `transact` without recipients fails.
- `transact` with recipients encrypts payload.

**Priority:** Critical
EOF
)" "$M1" "priority: critical" "phase1" "module: posvault" "type: feature"

create_issue "Add convenience API for default recipients" \
"$(cat <<'EOF'
**Goal:** Simplify setting recipients at vault level.

**Module:** `posvault`
**Type:** feature

**Deliverables:**
- `set_recipients` and `with_recipients` builder-style.

**Acceptance Criteria:**
- Users can set recipients once and use them for all transactions.

**Priority:** Critical
EOF
)" "$M1" "priority: critical" "phase1" "module: posvault" "type: feature"

# Default signing
create_issue "Require signer for event and journal mutations" \
"$(cat <<'EOF'
**Goal:** Ensure every mutation is signed.

**Module:** `posvault`, `posvault_sign`
**Type:** feature

**Deliverables:**
- `PosVault::transact` and `journal` error if no signer set.
- Error variant `Auth` or `Encryption`.

**Acceptance Criteria:**
- Unsigned mutations are rejected.
- Signed mutations are stored with valid signature.

**Priority:** Critical
EOF
)" "$M1" "priority: critical" "phase1" "module: posvault" "type: feature"

create_issue "Enable strict signature verification on read" \
"$(cat <<'EOF'
**Goal:** Prevent tampered data from being returned.

**Module:** `posvault_sign`
**Type:** feature

**Deliverables:**
- Default to strict mode in `SignedEventStore` and `SignedJournal`.
- Return error on verification failure.

**Acceptance Criteria:**
- Tampered event or journal entry is rejected.
- Valid entries pass.

**Priority:** Critical
EOF
)" "$M1" "priority: critical" "phase1" "module: posvault_sign" "type: feature"

# -----------------------------------------------------------------------------
# Phase 2 - Security & Audit
# -----------------------------------------------------------------------------

# RBAC
create_issue "Define role permission matrix" \
"$(cat <<'EOF'
**Goal:** Document which roles can perform which operations.

**Module:** `posvault_auth`
**Type:** docs

**Deliverables:**
- Permission table in README or docs
- Code constants for permissions.

**Acceptance Criteria:**
- Clear matrix exists.
- All operations mapped.

**Priority:** High
EOF
)" "$M2" "priority: high" "phase2" "module: posvault_auth" "type: docs"

create_issue "Implement `require_role` in PosVault facade" \
"$(cat <<'EOF'
**Goal:** Enforce RBAC at high-level API.

**Module:** `posvault`
**Type:** feature

**Deliverables:**
- Add role checks to `transact`, `journal`, `sync_to_remote`, `query_engine`.

**Acceptance Criteria:**
- Unauthorized role receives error.
- Authorized role succeeds.

**Priority:** High
EOF
)" "$M2" "priority: high" "phase2" "module: posvault" "type: feature"

# Audit trail append-only
create_issue "Ensure Journal API has no update/delete methods" \
"$(cat <<'EOF'
**Goal:** Guarantee journal immutability.

**Module:** `posvault_store`
**Type:** feature

**Deliverables:**
- Review `Journal` trait and implementations
- Remove or hide any mutation methods.

**Acceptance Criteria:**
- No public delete/update for journal entries.
- Compiler rejects attempts to modify.

**Priority:** High
EOF
)" "$M2" "priority: high" "phase2" "module: posvault_store" "type: feature"

# Key management
create_issue "Encrypt private key storage" \
"$(cat <<'EOF'
**Goal:** Never store private keys in plaintext.

**Module:** `posvault_crypto`, `posvault_auth`
**Type:** feature

**Deliverables:**
- Use `age` passphrase encryption for private keys.
- Store encrypted key and metadata.

**Acceptance Criteria:**
- Private key file is encrypted.
- Decryption with passphrase recovers key.

**Priority:** High
EOF
)" "$M2" "priority: high" "phase2" "module: posvault_crypto" "type: feature"

create_issue "Add key recovery API" \
"$(cat <<'EOF'
**Goal:** Allow recover if passphrase known.

**Module:** `posvault_auth`
**Type:** feature

**Deliverables:**
- `recover_key(passphrase)` function.
- Integration with login backend.

**Acceptance Criteria:**
- Correct passphrase recovers key.
- Wrong passphrase fails.

**Priority:** High
EOF
)" "$M2" "priority: high" "phase2" "module: posvault_auth" "type: feature"

# Anti-tamper detection
create_issue "Implement event tamper detection tests" \
"$(cat <<'EOF'
**Goal:** Prove modified data is rejected.

**Module:** `posvault_sign`
**Type:** test

**Deliverables:**
- Tests that mutate payload/signature and expect verification failure.

**Acceptance Criteria:**
- Tests pass and demonstrate detection.

**Priority:** High
EOF
)" "$M2" "priority: high" "phase2" "module: posvault_sign" "type: test"

# Sync conflict resolution
create_issue "Implement 3-way merge algorithm for tree sync" \
"$(cat <<'EOF'
**Goal:** Merge two divergent histories without conflict.

**Module:** `posvault_sync`
**Type:** feature

**Deliverables:**
- Find common ancestor (base)
- Compare trees base/ours/theirs
- If no overlapping changes, merge simply.

**Acceptance Criteria:**
- Non-conflicting trees merge correctly.
- Conflicts produce descriptive error.

**Priority:** High
EOF
)" "$M2" "priority: high" "phase2" "module: posvault_sync" "type: feature"

create_issue "Integrate conflict resolver into FileTransport" \
"$(cat <<'EOF'
**Goal:** Use resolver during pull/push.

**Module:** `posvault_sync`
**Type:** feature

**Deliverables:**
- `pull` invokes resolver if both sides changed.
- Return merged result or error.

**Acceptance Criteria:**
- Pull works with non-conflicting changes.
- Conflict error is returned.

**Priority:** High
EOF
)" "$M2" "priority: high" "phase2" "module: posvault_sync" "type: feature"

# -----------------------------------------------------------------------------
# Phase 3 - POS Operational
# -----------------------------------------------------------------------------

# Branch per store
create_issue "Use active branch in PosVault::transact" \
"$(cat <<'EOF'
**Goal:** Transactions should target current branch.

**Module:** `posvault`
**Type:** feature

**Deliverables:**
- `transact` reads active branch from FileStore HEAD.
- Commits appended to that branch.

**Acceptance Criteria:**
- After checkout, events go to new branch.
- Main branch unaffected.

**Priority:** Medium
EOF
)" "$M3" "priority: medium" "phase3" "module: posvault" "type: feature"

# Stock management
create_issue "Implement update_stock function" \
"$(cat <<'EOF'
**Goal:** Change stock quantity via events.

**Module:** `posvault_query`
**Type:** feature

**Deliverables:**
- `update_stock(item, delta)` creates event.
- Validate non-negative stock.

**Acceptance Criteria:**
- Stock updated and persisted.
- Negative stock rejected.

**Priority:** Medium
EOF
)" "$M3" "priority: medium" "phase3" "module: posvault_query" "type: feature"

create_issue "Implement stock_history query" \
"$(cat <<'EOF'
**Goal:** Retrieve all changes for an item.

**Module:** `posvault_query`
**Type:** feature

**Deliverables:**
- `stock_history(item) -> Vec<(timestamp, delta)>`
- Uses event store checkpoint.

**Acceptance Criteria:**
- History correct and ordered.

**Priority:** Medium
EOF
)" "$M3" "priority: medium" "phase3" "module: posvault_query" "type: feature"

# Daily sales
create_issue "Implement daily_sales reporting" \
"$(cat <<'EOF'
**Goal:** Report total sales per day.

**Module:** `posvault_query`
**Type:** feature

**Deliverables:**
- `daily_sales(date) -> Result<u64>`
- Aggregate event payloads.

**Acceptance Criteria:**
- Total matches sum of events for date.
- Error if date not found.

**Priority:** Medium
EOF
)" "$M3" "priority: medium" "phase3" "module: posvault_query" "type: feature"

# Backup and restore
create_issue "Implement store export functionality" \
"$(cat <<'EOF'
**Goal:** Export whole vault to archive/file.

**Module:** `posvault_store`
**Type:** feature

**Deliverables:**
- `export(dest_path)` copies all objects and refs.
- Deterministic output.

**Acceptance Criteria:**
- Export is complete.
- Can be imported without loss.

**Priority:** Medium
EOF
)" "$M3" "priority: medium" "phase3" "module: posvault_store" "type: feature"

create_issue "Implement store import and restore" \
"$(cat <<'EOF'
**Goal:** Restore from export.

**Module:** `posvault_store`
**Type:** feature

**Deliverables:**
- `import(src_path)` loads objects and refs.
- Verify integrity after restore.

**Acceptance Criteria:**
- Restored store hashes match original.

**Priority:** Medium
EOF
)" "$M3" "priority: medium" "phase3" "module: posvault_store" "type: feature"

# CLI
create_issue "Implement basic CLI with clap" \
"$(cat <<'EOF'
**Goal:** Provide manual testing and demo commands.

**Module:** root crate
**Type:** feature

**Deliverables:**
- Subcommands: `init`, `login`, `add-event`, `list-events`, `sync`.
- Use existing PosVault facade.

**Acceptance Criteria:**
- CLI runs successfully.
- Output is informative.

**Priority:** Low
EOF
)" "$M3" "priority: low" "phase3" "module: posvault" "type: feature"

echo ""
echo "All milestones, labels, and issues have been created."
