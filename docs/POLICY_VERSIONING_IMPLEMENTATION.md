# Policy Versioning Implementation

**Status**: ✅ Backend Complete, Frontend Pending (Task 2.4 of Phase 2)
**Date**: October 7, 2025

---

## Overview

Implemented comprehensive policy versioning system that tracks every policy change, provides full audit history, and associates runs with the policy version active when they were created.

### Problem Solved

**Before**:
- Policy changes overwrote previous versions
- No history or audit trail
- Impossible to see what policy was active for historical runs
- No attribution for who changed what
- No rollback capability

**After**:
- Every policy change creates a new version
- Complete history with timestamps and attribution
- Runs record which policy version was active
- Optional change notes for documentation
- Full audit trail for compliance

---

## Architecture

### Database Schema

**New Table: `policy_versions`**
```sql
CREATE TABLE policy_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id TEXT NOT NULL,
    version INTEGER NOT NULL,          -- Incremental version number (1, 2, 3...)
    policy_json TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_by TEXT,                   -- Optional: user/system identifier
    change_notes TEXT,                  -- Optional: description of changes
    FOREIGN KEY (project_id) REFERENCES projects(id),
    UNIQUE(project_id, version)
);
```

**Updated Table: `policies`**
```sql
ALTER TABLE policies ADD COLUMN current_version INTEGER NOT NULL DEFAULT 1;
```

**Updated Table: `runs`**
```sql
ALTER TABLE runs ADD COLUMN policy_version INTEGER;
```

### How It Works

1. **Version Creation**: Every time a policy is updated, a new version entry is created
2. **Current Version Tracking**: The `policies` table tracks the current version number
3. **Run Association**: When a run is created, it records the current policy version
4. **History Preservation**: Old versions remain in `policy_versions` table forever

### Example Version History

| Version | Created At | Created By | Budget Tokens | Change Notes |
|---------|-----------|------------|---------------|--------------|
| 1 | 2025-10-01 10:00 | system | 1000 | Initial policy |
| 2 | 2025-10-02 14:30 | user | 5000 | Increased for production |
| 3 | 2025-10-03 09:15 | user | 2000 | Reduced after budget review |

---

## Implementation Details

### 1. Database Migration (V14)

**File**: `src-tauri/src/store/migrations/V14__policy_versioning.sql`

- Creates `policy_versions` table
- Adds `current_version` to `policies`
- Adds `policy_version` to `runs`
- Migrates existing policies to version 1

### 2. Rust Backend

**PolicyVersion Struct** (`src-tauri/src/store/policies.rs`):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyVersion {
    pub id: i64,
    pub project_id: String,
    pub version: i64,
    pub policy: Policy,
    pub created_at: String,
    pub created_by: Option<String>,
    pub change_notes: Option<String>,
}
```

**New Functions** (`src-tauri/src/store/policies.rs`):

```rust
// Create new version with optional notes
pub fn upsert_with_notes(
    conn: &Connection,
    project_id: &str,
    policy: &Policy,
    created_by: Option<&str>,
    change_notes: Option<&str>,
) -> Result<(), Error>

// Get all versions for a project
pub fn get_versions(
    conn: &Connection,
    project_id: &str,
) -> Result<Vec<PolicyVersion>, Error>

// Get specific version
pub fn get_version(
    conn: &Connection,
    project_id: &str,
    version: i64,
) -> Result<Option<PolicyVersion>, Error>

// Get current version number
pub fn get_current_version(
    conn: &Connection,
    project_id: &str,
) -> Result<i64, Error>
```

### 3. API Commands

**New Commands** (`src-tauri/src/api.rs`):

```rust
#[tauri::command]
pub fn update_policy_with_notes(
    project_id: String,
    policy: Policy,
    change_notes: Option<String>,
    pool: State<'_, DbPool>,
) -> Result<(), Error>

#[tauri::command]
pub fn get_policy_versions(
    project_id: String,
    pool: State<'_, DbPool>,
) -> Result<Vec<store::policies::PolicyVersion>, Error>

#[tauri::command]
pub fn get_policy_version(
    project_id: String,
    version: i64,
    pool: State<'_, DbPool>,
) -> Result<Option<store::policies::PolicyVersion>, Error>

#[tauri::command]
pub fn get_current_policy_version_number(
    project_id: String,
    pool: State<'_, DbPool>,
) -> Result<i64, Error>
```

### 4. Run Association

**Updated** `start_run` (`src-tauri/src/orchestrator.rs`):

```rust
// Get current policy version before creating run
let policy_version = crate::store::policies::get_current_version(&conn, project_id).ok();

// Include in INSERT statement
tx.execute(
    "INSERT INTO runs (..., policy_version) VALUES (..., ?11)",
    params![..., policy_version],
)?;
```

Now runs track which policy was active when they were created.

---

## Files Modified

1. **`src-tauri/src/store/migrations/V14__policy_versioning.sql`** (NEW)
   - Database schema changes
   - Migration logic for existing policies

2. **`src-tauri/src/store/migrations.rs`** (+1 line)
   - Registered V14 migration

3. **`src-tauri/src/store/policies.rs`** (+120 lines)
   - Added `PolicyVersion` struct
   - Added `upsert_with_notes()` function
   - Added `get_versions()` function
   - Added `get_version()` function
   - Added `get_current_version()` function
   - Updated `upsert()` to delegate to `upsert_with_notes()`

4. **`src-tauri/src/orchestrator.rs`** (+3 lines)
   - Query current policy version when creating run
   - Include policy_version in run INSERT statement

5. **`src-tauri/src/api.rs`** (+45 lines)
   - Added `update_policy_with_notes()` command
   - Added `get_policy_versions()` command
   - Added `get_policy_version()` command
   - Added `get_current_policy_version_number()` command

6. **`src-tauri/src/main.rs`** (+8 lines)
   - Registered new API commands in both feature-gated invoke handlers

---

## Usage

### Creating a Policy Version with Notes

```typescript
await invoke('update_policy_with_notes', {
  projectId: 'project-123',
  policy: {
    allowNetwork: true,
    budgetTokens: 5000,
    budgetUsd: 10.0,
    budgetNatureCost: 100.0,
  },
  changeNotes: 'Increased token budget for production deployment',
});
```

### Getting Policy History

```typescript
const versions = await invoke('get_policy_versions', {
  projectId: 'project-123',
});

// Returns:
[
  {
    id: 3,
    projectId: 'project-123',
    version: 3,
    policy: { ... },
    createdAt: '2025-10-03T09:15:00Z',
    createdBy: 'user',
    changeNotes: 'Reduced after budget review',
  },
  {
    id: 2,
    projectId: 'project-123',
    version: 2,
    policy: { ... },
    createdAt: '2025-10-02T14:30:00Z',
    createdBy: 'user',
    changeNotes: 'Increased for production',
  },
  {
    id: 1,
    projectId: 'project-123',
    version: 1,
    policy: { ... },
    createdAt: '2025-10-01T10:00:00Z',
    createdBy: 'system',
    changeNotes: 'Initial policy',
  },
]
```

### Getting a Specific Version

```typescript
const version2 = await invoke('get_policy_version', {
  projectId: 'project-123',
  version: 2,
});
```

### Checking Current Version Number

```typescript
const currentVersion = await invoke('get_current_policy_version_number', {
  projectId: 'project-123',
});
// Returns: 3
```

---

## Frontend Tasks (TODO)

### 1. TypeScript Types

**Add to `app/src/lib/api.ts`**:
```typescript
export interface PolicyVersion {
  id: number;
  projectId: string;
  version: number;
  policy: Policy;
  createdAt: string;
  createdBy?: string | null;
  changeNotes?: string | null;
}
```

### 2. UI Components

**Policy History Panel**:
- Display list of all policy versions
- Show version number, timestamp, change notes
- Highlight current version
- Allow viewing old version details
- Optional: Compare versions side-by-side

**ContextPanel Enhancement**:
- Add "View History" button next to policy settings
- Show current version number
- Optional: Add change notes field when saving policy
- Show policy version for currently selected run

**Example UI**:
```
┌─ Policy Settings ─────────────────────────┐
│                                            │
│  Current Version: 3  [View History]        │
│                                            │
│  Token Budget: [5000]                      │
│  USD Budget: [$10.00]                      │
│  Nature Cost: [100.0]                      │
│  Allow Network: [✓]                        │
│                                            │
│  Change Notes (optional):                  │
│  [____________________________________]    │
│                                            │
│  [Save Policy]                             │
└────────────────────────────────────────────┘
```

**Policy History Modal**:
```
┌─ Policy History ──────────────────────────┐
│                                            │
│  Version 3 (Current)                       │
│  2025-10-03 09:15 AM  by user              │
│  "Reduced after budget review"             │
│  Tokens: 2000 | USD: $5.00                 │
│  ───────────────────────────────────────   │
│  Version 2                                 │
│  2025-10-02 02:30 PM  by user              │
│  "Increased for production"                │
│  Tokens: 5000 | USD: $10.00                │
│  ───────────────────────────────────────   │
│  Version 1                                 │
│  2025-10-01 10:00 AM  by system            │
│  "Initial policy"                          │
│  Tokens: 1000 | USD: $1.00                 │
│                                            │
│  [Close]                                   │
└────────────────────────────────────────────┘
```

---

## Use Cases

### 1. Compliance Audit

**Scenario**: Auditor asks "What policy was active when run X executed?"

**Solution**:
```sql
SELECT r.id, r.created_at, r.policy_version, pv.policy_json, pv.change_notes
FROM runs r
LEFT JOIN policy_versions pv ON r.policy_version = pv.version AND pv.project_id = r.project_id
WHERE r.id = 'run-x';
```

Shows exactly what budgets/restrictions were active for that run.

### 2. Policy Experimentation

**Scenario**: User wants to try stricter policy but keep history

**Workflow**:
1. View current policy (v2: tokens=5000)
2. Update policy with notes (v3: tokens=2000, "Testing stricter limits")
3. Execute runs, see if they complete
4. If too strict, update again (v4: tokens=3000, "Relaxed slightly")
5. Full history preserved for review

### 3. Incident Investigation

**Scenario**: Run failed due to budget violation, need to understand why

**Investigation**:
1. Check run's `policy_version` field
2. Look up that version in `policy_versions`
3. See the exact budgets that were active
4. Check change_notes to understand rationale

---

## Benefits

### For Users

✅ **Transparency**: See why policy changed and when
✅ **Safety**: Can't accidentally lose policy history
✅ **Documentation**: Change notes provide context
✅ **Accountability**: Know who made changes (when auth added)

### For Developers

✅ **Debugging**: Reproduce historical behavior exactly
✅ **Testing**: Compare results under different policies
✅ **Experimentation**: Try changes without fear of data loss

### For Auditors

✅ **Compliance**: Full audit trail of policy changes
✅ **Reproducibility**: Know exact policy for any run
✅ **Transparency**: No hidden changes or deletions

---

## Future Enhancements

### Phase 3 Features

1. **Policy Rollback**
   - UI button to revert to previous version
   - "Restore Version 2" → creates Version 5 with v2's values
   - Change notes: "Restored from version 2"

2. **Policy Diff Viewer**
   - Side-by-side comparison of two versions
   - Highlight what changed (e.g., "Tokens: 1000 → 5000")
   - Visual diff like git

3. **Policy Templates**
   - Save common policies as templates
   - "Conservative", "Development", "Production"
   - Quick apply with version creation

4. **Authentication Integration**
   - Track actual user IDs instead of "user"
   - Show user names in history
   - Permission controls (who can change policy)

5. **Policy Schedules**
   - Different policies for different times
   - "Weekday: strict, Weekend: permissive"
   - Automatic version creation

6. **Export/Import**
   - Include policy history in IXP archives
   - Share policy evolution with collaborators

---

## Testing Instructions

### Test 1: Migration

**Steps**:
1. Restart app to trigger migration
2. Check database:
   ```bash
   sqlite3 ~/.local/share/com.intelexta.dev/*/intelexta.db "SELECT * FROM policy_versions"
   ```

**Expected**:
- Existing policy migrated to version 1
- `created_by = 'system'`
- Change notes mention migration

### Test 2: Version Creation

**Steps**:
1. Update policy via ContextPanel
2. Check database:
   ```bash
   sqlite3 ~/.local/share/com.intelexta.dev/*/intelexta.db "SELECT version, created_at, change_notes FROM policy_versions ORDER BY version"
   ```

**Expected**:
- New version 2 created
- Timestamp is recent
- Both v1 and v2 exist

### Test 3: Run Association

**Steps**:
1. Create and execute a run
2. Check run's policy_version:
   ```bash
   sqlite3 ~/.local/share/com.intelexta.dev/*/intelexta.db "SELECT id, policy_version FROM runs ORDER BY created_at DESC LIMIT 1"
   ```

**Expected**:
- Run has `policy_version = 2` (or current version)

### Test 4: History Retrieval

**Steps**:
1. Open browser console
2. Get current project ID
3. Execute:
   ```javascript
   await invoke('get_policy_versions', { projectId: '<project-id>' })
   ```

**Expected**:
- Array of all versions returned
- Newest first (descending order)
- Each has full policy data

---

## Summary

Policy versioning provides:

- **Complete Audit Trail**: Every change tracked forever
- **Run Association**: Know what policy was active for any run
- **Change Attribution**: See who changed what and why
- **History Preservation**: Never lose policy configurations
- **Compliance Ready**: Full transparency for audits

**Implementation Time**: ~2 hours (backend only)
**Files Modified**: 6
**Lines Added**: ~180
**Migration**: V14 (auto-applies on restart)

✅ **Backend Complete**
⏳ **Frontend UI Pending**
