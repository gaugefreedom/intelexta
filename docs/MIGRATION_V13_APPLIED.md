# Migration V13 - Database Update Required

**Issue**: `table checkpoint_payloads has no column named full_output_hash`

**Cause**: The database migration V13 was not applied to your running database.

**Solution**: Restart the application to apply the migration automatically.

## Steps to Apply Migration

### Option 1: Restart the Application (Recommended)

1. **Close the application** completely
2. **Rebuild the backend**:
   ```bash
   cd src-tauri
   cargo build
   ```
3. **Restart the application**:
   ```bash
   npm run tauri dev
   ```
4. **Verify migration applied**:
   - The migration will run automatically on startup
   - Check console for migration messages
   - Try creating and executing a run

### Option 2: Manual Database Migration (Advanced)

If you need to apply the migration manually:

```bash
# Connect to the database
sqlite3 ~/.local/share/intelexta/intelexta.sqlite

# Check current migration version
SELECT MAX(version) FROM migrations;

# If less than 13, manually run:
ALTER TABLE checkpoint_payloads ADD COLUMN full_output_hash TEXT;
CREATE INDEX IF NOT EXISTS idx_checkpoint_payloads_hash ON checkpoint_payloads(full_output_hash);

# Record migration
INSERT INTO migrations (version) VALUES (13);

# Exit
.quit
```

### Option 3: Fresh Database (Clean Slate)

**WARNING**: This will delete all existing data!

```bash
# Backup old database
mv ~/.local/share/intelexta/intelexta.sqlite ~/.local/share/intelexta/intelexta.sqlite.backup

# Restart app - fresh database will be created with all migrations
npm run tauri dev
```

## Verification

After restart, you should see:
- ✅ No errors about "full_output_hash" column
- ✅ Runs execute successfully
- ✅ Checkpoint details show "Download Full Output" button
- ✅ Attachments directory created at `~/.local/share/intelexta/attachments/`

## What Changed

**Migration V13** adds:
- New column `full_output_hash` to `checkpoint_payloads` table
- Index on `full_output_hash` for performance
- Enables attachment store functionality

**Files Modified**:
- `src-tauri/src/store/migrations.rs` - Added V12 and V13 to migration list
- `src-tauri/src/store/migrations/V13__add_full_output_hash.sql` - Migration script

## Testing After Migration

1. **Create a new run** with Ingest Document step
2. **Execute the run**
3. **Open Inspector** → select checkpoint
4. **Verify**:
   - Preview shows in database (first 1000 chars)
   - "Download Full Output" button appears
   - Clicking button downloads complete output
   - Check `~/.local/share/intelexta/attachments/` for files

## Troubleshooting

### Migration still not applied after restart

Check database directly:
```bash
sqlite3 ~/.local/share/intelexta/intelexta.sqlite "SELECT MAX(version) FROM migrations;"
```

Should return: `13`

If it returns less than 13, check console logs for migration errors.

### "attachments directory not initialized" error

The app should initialize the attachments directory automatically. If not:
```bash
mkdir -p ~/.local/share/intelexta/attachments
```

Then restart the app.

---

**Ready**: Restart the app now to apply the migration!
