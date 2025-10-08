# Button Debugging Guide - Checkpoint Details Panel

**Issue**: Copy, Download, and Download Full Output buttons not responding when clicked.

**Changes Made**: Added comprehensive console logging to all button click handlers.

---

## Testing Instructions

### Step 1: Rebuild and Restart

```bash
cd /home/marcelo/Documents/codes/gaugefreedom/intelexta
npm run tauri dev
```

### Step 2: Open Browser DevTools

1. When the app opens, press `F12` or `Ctrl+Shift+I`
2. Go to the **Console** tab
3. Keep it open during testing

### Step 3: Execute a Run and Open Checkpoint

1. Create a run with an Ingest Document step
2. Execute the run
3. Open Inspector
4. Click on a checkpoint to open the details panel

### Step 4: Test Each Button

#### Test A: Copy Button (in PayloadViewer)

**Location**: Above the output text, next to "Raw/Canonical/Digest" toggles

**Steps**:
1. Make sure you're in "Raw" view (click "Raw" button if needed)
2. Click the "Copy" button
3. **Check console** for these messages:
   ```
   Copy clicked - label: Output, viewMode: raw, disabled: false
   Copied to clipboard successfully
   ```

**If you see**:
- `disabled: true` → Button is correctly disabled (no data in that view)
- `disabled: false` → Button should work, check for errors

#### Test B: Download Button (in PayloadViewer)

**Location**: Next to the Copy button

**Steps**:
1. Make sure you're in "Raw" view
2. Click the "Download" button
3. **Check console** for:
   ```
   Download clicked - label: Output, viewMode: raw, disabled: false
   Downloading file: <checkpoint-id>-output-raw.txt
   Download triggered successfully
   ```

**Expected**: A file should download to your Downloads folder

#### Test C: Download Full Output Button

**Location**: Below the output text, separate button

**Steps**:
1. Click the "Download Full Output" button
2. **Check console** for:
   ```
   Download Full Output clicked, checkpoint ID: <uuid>
   Calling download_checkpoint_full_output...
   Received output, length: <number>
   Download triggered successfully
   ```

**Expected**: A file named `checkpoint-<id>-full-output.txt` should download

---

## Diagnostic Scenarios

### Scenario 1: No Console Messages At All

**Symptoms**: Click button, nothing in console

**Cause**: Click handler not attached OR React not re-rendering

**Check**:
- Is the button visible?
- Try refreshing the page (`Ctrl+R`)
- Check if there are any React errors in console

### Scenario 2: "disabled: true" Message

**Symptoms**: Console shows `Copy/Download is disabled, returning early`

**Cause**: No data for that view mode

**Check**:
- What view are you in? (Raw/Canonical/Digest)
- Does the output text say "No raw content stored..."?
- Try switching to "Raw" view first

### Scenario 3: Download Full Output - No Response

**Symptoms**: Console shows first log but nothing after

**Cause**: Tauri `invoke` call failing silently

**Check**:
- Look for **Rust backend errors** in the terminal where you ran `npm run tauri dev`
- Should see something like:
  ```
  Error: API Error: No full output attachment found for checkpoint <id>
  ```

### Scenario 4: Download Full Output - API Error

**Symptoms**: Console shows error message

**Possible Errors**:

**A. "No full output attachment found"**
```
Download Full Output error: API Error: No full output attachment found for checkpoint <id>
```

**Cause**: Checkpoint created before migration V13 applied

**Solution**: Create a NEW run after migration applied, then test again

**B. "Failed to load attachment"**
```
Download Full Output error: API Error: Failed to load attachment: <hash>
```

**Cause**: Attachment file missing or corrupted

**Check**:
```bash
# See what attachments exist
ls -lh ~/.local/share/com.intelexta.dev/attachments/*/
```

**C. "checkpoint not found"**
```
Download Full Output error: API Error: checkpoint not found
```

**Cause**: Database query failed

**Solution**: Check database migration version

---

## Expected Console Output (Full Success)

When everything works, you should see:

```
# When clicking Copy (Raw view)
Copy clicked - label: Output, viewMode: raw, disabled: false
Copied to clipboard successfully

# When clicking Download (Raw view)
Download clicked - label: Output, viewMode: raw, disabled: false
Downloading file: abc123-output-raw.txt
Download triggered successfully

# When clicking Download Full Output
Download Full Output clicked, checkpoint ID: abc-123-def-456
Calling download_checkpoint_full_output...
Received output, length: 2734
Download triggered successfully
```

---

## Common Issues & Solutions

### Issue: Buttons appear grayed out

**Cause**: They're disabled via CSS

**Check**: Look at button styling - if `opacity: 0.5` and `cursor: not-allowed`, they're disabled

**Solution**: Switch to "Raw" view where data exists

### Issue: Clipboard permission denied

**Symptoms**: Console shows "Clipboard API not available" or permission error

**Cause**: Browser security restrictions

**Solution**:
- Tauri should have clipboard access by default
- If not, check Tauri config for clipboard permissions

### Issue: Download triggered but no file appears

**Symptoms**: Console shows "Download triggered successfully" but no file in Downloads

**Check**:
- Browser download settings
- Check if download was blocked (browser notification)
- Try a different download location

---

## Data Verification

After successful download of full output, verify it contains complete data:

```bash
# Check file size
ls -lh ~/Downloads/checkpoint-*-full-output.txt

# View first few lines
head -20 ~/Downloads/checkpoint-*-full-output.txt

# Count characters
wc -c ~/Downloads/checkpoint-*-full-output.txt
```

Compare to what's in the attachment store:

```bash
# Find the attachment
find ~/.local/share/com.intelexta.dev/attachments -name "*.txt" -exec ls -lh {} \;

# View one
cat ~/.local/share/com.intelexta.dev/attachments/03/031cc0b99e1c215d424d5703e9a46f545d3093a916316922c87f3783a41fb357.txt
```

They should match exactly.

---

## Next Steps

### If Copy/Download work in Raw view:

✅ Frontend button handlers are working correctly
✅ Buttons are properly checking `copyDisabled`
✅ Issue was just view mode selection

### If Download Full Output works:

✅ Tauri invoke working
✅ Backend API command working
✅ Attachment store working
✅ Database migration applied
✅ **Ready for Task 2.2: CAR Export**

### If still not working:

Report the exact console output and I'll debug further.

---

## Testing Checklist

- [ ] Console open during testing
- [ ] Checkpoint details panel open
- [ ] Tried clicking Copy in "Raw" view
- [ ] Tried clicking Download in "Raw" view
- [ ] Tried clicking Download Full Output
- [ ] Noted all console messages
- [ ] Checked terminal for Rust errors
- [ ] Verified file downloads (if any)

**Report back with console output and I'll help debug!**
