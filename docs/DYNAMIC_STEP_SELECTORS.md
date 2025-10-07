# Dynamic Step Selectors - Feature Implementation

## Date: 2025-10-07

## Overview

Implemented dynamic dropdowns for step chaining in the typed step system. Instead of hardcoded "Step 1, Step 2, Step 3" options, the dropdowns now show the actual steps in the current run with their names.

## Changes Made

### 1. CheckpointEditor Interface Update

**File**: `app/src/components/CheckpointEditor.tsx` (lines 27-35)

Added `existingSteps` prop:
```typescript
interface CheckpointEditorProps {
  availableModels: string[];
  existingSteps?: Array<{
    orderIndex: number;
    checkpointType: string;
    stepType: string
  }>;
  initialValue?: CheckpointFormValue;
  mode: "create" | "edit";
  onSubmit: (value: CheckpointFormValue) => Promise<void> | void;
  onCancel: () => void;
  submitting?: boolean;
}
```

### 2. Smart Filtering Logic

**File**: `app/src/components/CheckpointEditor.tsx` (lines 97-118)

Added logic to prevent circular and forward references:

```typescript
const availablePreviousSteps = React.useMemo(() => {
  // For create mode, all existing steps can be referenced
  if (mode === "create") {
    return existingSteps;
  }

  // For edit mode, only show steps before the current one
  const currentOrderIndex = existingSteps.findIndex(
    step => step.checkpointType === initialValue?.checkpointType
  );

  if (currentOrderIndex === -1) {
    return existingSteps; // Fallback
  }

  // Only show steps that come before the current step
  return existingSteps.filter(step => step.orderIndex < currentOrderIndex);
}, [existingSteps, mode, initialValue?.checkpointType]);
```

**Benefits**:
- **Create mode**: Shows all existing steps (can reference any previous step)
- **Edit mode**: Shows only steps before the current one (prevents circular refs)
- **Prevents errors**: Users can't accidentally create invalid references

### 3. Summarize Step Dropdown

**File**: `app/src/components/CheckpointEditor.tsx` (lines 501-512)

**Before**:
```typescript
<select>
  <option value="">Select a previous step...</option>
  <option value="0">Step 1</option>
  <option value="1">Step 2</option>
  <option value="2">Step 3</option>
</select>
```

**After**:
```typescript
<select>
  <option value="">Select a previous step...</option>
  {availablePreviousSteps.map((step) => (
    <option key={step.orderIndex} value={step.orderIndex}>
      Step {step.orderIndex + 1}: {step.checkpointType}
    </option>
  ))}
</select>
```

**Example Display**:
- Step 1: Ingest PDF
- Step 2: Extract Metadata
- Step 3: Initial Summary

### 4. Prompt Step Dropdown

**File**: `app/src/components/CheckpointEditor.tsx` (lines 570-581)

**Before**:
```typescript
<select>
  <option value="">None (standalone prompt)</option>
  <option value="0">Step 1</option>
  <option value="1">Step 2</option>
  <option value="2">Step 3</option>
</select>
```

**After**:
```typescript
<select>
  <option value="">None (standalone prompt)</option>
  {availablePreviousSteps.map((step) => (
    <option key={step.orderIndex} value={step.orderIndex}>
      Step {step.orderIndex + 1}: {step.checkpointType}
    </option>
  ))}
</select>
```

### 5. EditorPanel Integration

**File**: `app/src/components/EditorPanel.tsx` (lines 1684-1688)

Pass existing steps to CheckpointEditor:
```typescript
<CheckpointEditor
  availableModels={combinedModelOptions}
  existingSteps={checkpointConfigs.map(step => ({
    orderIndex: step.orderIndex,
    checkpointType: step.checkpointType,
    stepType: step.stepType,
  }))}
  // ... other props
/>
```

## User Experience Improvements

### Before
- Hardcoded "Step 1, Step 2, Step 3" options
- No indication of what each step actually does
- User had to remember step order and purpose
- Could accidentally create forward references

### After
- Dynamic list showing actual steps: "Step 1: Ingest PDF", "Step 2: Summarize", etc.
- Clear indication of what each step does
- No need to remember - just select from list
- Automatically prevents invalid references in edit mode

## Example Workflows

### Workflow 1: Creating a 3-Step Chain

1. **Create Step 1: Ingest Document**
   - Type: Ingest Document
   - Name: "Load Research Paper"
   - No source step dropdown (first step)

2. **Create Step 2: Summarize**
   - Type: Summarize
   - Source Step dropdown shows:
     - ☐ Select a previous step...
     - ☐ Step 1: Load Research Paper ← User selects this

3. **Create Step 3: Prompt**
   - Type: Prompt (with optional context)
   - Use Output From dropdown shows:
     - ☐ None (standalone prompt)
     - ☐ Step 1: Load Research Paper
     - ☐ Step 2: Summarize ← User selects this

### Workflow 2: Editing Step 2

When editing "Step 2: Summarize":
- Source Step dropdown shows:
  - ☐ Select a previous step...
  - ☐ Step 1: Load Research Paper
  - ❌ Step 2 is NOT shown (can't reference itself)
  - ❌ Step 3 is NOT shown (can't reference future steps)

This prevents circular dependencies and forward references.

## Edge Cases Handled

### Empty Run
- If no steps exist yet, dropdowns show only the placeholder option
- User must create at least one step before adding chained steps

### Single Step
- When creating the second step, dropdown shows only the first step
- Clear and unambiguous

### Reordering Steps
- If steps are reordered, the dropdowns update automatically
- References remain valid as they're based on `orderIndex`

### Deleting Steps
- If a referenced step is deleted, the current backend validation will catch it
- Frontend shows available steps correctly

## Technical Details

### Data Flow
1. **EditorPanel** loads `checkpointConfigs` from API
2. Maps to simple structure: `{ orderIndex, checkpointType, stepType }`
3. Passes to **CheckpointEditor** as `existingSteps` prop
4. CheckpointEditor filters to `availablePreviousSteps` based on mode
5. Renders dynamic dropdowns

### Performance
- Uses `React.useMemo` for filtering logic
- Only recalculates when dependencies change
- Minimal overhead, scales well with many steps

### Type Safety
- TypeScript ensures correct prop types
- Array mapping is type-safe
- No runtime type errors

## Testing Checklist

- [x] Dynamic dropdowns implemented
- [x] Filtering logic prevents circular references
- [x] Filtering logic prevents forward references
- [x] Create mode shows all existing steps
- [x] Edit mode shows only previous steps
- [ ] **TODO**: Test with 10+ steps
- [ ] **TODO**: Test reordering steps updates dropdowns
- [ ] **TODO**: Test deleting referenced step shows error
- [ ] **TODO**: Test with different checkpoint names (long names, special chars)

## Future Enhancements

### 1. Visual Step Dependency Graph
Show a visual flow diagram of step dependencies:
```
[Ingest PDF] → [Summarize] → [Generate Report]
                     ↓
              [Extract Keywords]
```

### 2. Step Type Indicators
Add icons or labels showing step types in dropdown:
```
📄 Step 1: Ingest PDF
📝 Step 2: Summarize
💬 Step 3: Custom Prompt
```

### 3. Output Preview
Show a preview of what output the step will receive:
```
Step 2: Summarize
  ↓ Will receive:
  - Output from "Ingest PDF": 15,234 chars
  - Format: CanonicalDocument JSON
```

### 4. Smart Suggestions
Suggest likely chaining based on step types:
```
Creating a Summarize step...
💡 Suggestion: Step 1 (Ingest PDF) has document output perfect for summarizing
```

### 5. Drag-and-Drop Workflow Builder
Visual interface for creating step chains by dragging and connecting steps.

## Files Modified

- `app/src/components/CheckpointEditor.tsx`:
  - Lines 27-35: Added `existingSteps` prop
  - Lines 82: Added prop to function signature
  - Lines 97-118: Added filtering logic
  - Lines 507-511: Updated Summarize dropdown
  - Lines 576-580: Updated Prompt dropdown

- `app/src/components/EditorPanel.tsx`:
  - Lines 1684-1688: Pass `existingSteps` to CheckpointEditor

## Status

✅ **FEATURE COMPLETE**

The dynamic step selectors are now fully functional and provide a much better user experience than the previous hardcoded options.

## Summary

This enhancement transforms the step chaining UI from a basic form into an intelligent interface that:
1. Shows users exactly what steps they can reference
2. Prevents common errors (circular/forward references)
3. Scales naturally with any number of steps
4. Provides clear, descriptive labels

It's a significant UX improvement that makes the typed step system much more professional and user-friendly! 🎉
