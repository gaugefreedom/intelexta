# Typed Steps System - Complete Implementation

## Overview

The Typed Steps system enables chaining of AI workflow steps, where outputs from one step can be used as inputs to subsequent steps. This creates verifiable, reproducible multi-step workflows.

## What We Built

### Three Core Step Types

#### 1. **Ingest Document**
- **Purpose**: Load and process documents (PDF, LaTeX, TXT, DOCX)
- **Output**: Structured CanonicalDocument with extracted text
- **No LLM**: Pure document processing, deterministic
- **Use Case**: First step in document analysis workflows

#### 2. **Summarize**
- **Purpose**: Generate AI summaries of previous step outputs
- **Input**: Output from a previous step (required)
- **Types**: Brief, Detailed, Academic, Custom
- **Models**: Stub, Mock Claude, or Real LLM (Ollama)
- **Use Case**: Condense document content or previous results

#### 3. **Prompt (with optional context)**
- **Purpose**: Run custom LLM prompts with optional context
- **Input**: Output from a previous step (optional)
- **Models**: Stub, Mock Claude, or Real LLM (Ollama)
- **Use Case**: Ask questions, analyze results, chain reasoning

### Key Features Implemented

✅ **Step Chaining**: Output from Step N flows to Step N+1
✅ **Dynamic Dropdowns**: Shows actual step names, not hardcoded options
✅ **Smart Filtering**: Prevents circular and forward references
✅ **Model Routing**: Handles stub, mock, and real LLM models correctly
✅ **JSON Deserialization**: Frontend ↔ Backend communication works
✅ **Execution Tracking**: Prior outputs stored in HashMap for chaining
✅ **Debug Logging**: Clear visibility into what's happening

## How It Works

### Backend Architecture

**File**: `src-tauri/src/orchestrator.rs`

1. **StepConfig Enum** (lines 41-93):
   - Tagged enum with `stepType` discriminator
   - Uses camelCase serialization for frontend compatibility
   - Each variant has its own configuration structure

2. **Execution Flow** (lines 1604-1913):
   - Prior outputs stored in `HashMap<usize, StepOutput>`
   - Each step execution:
     1. Parses `config_json` as `StepConfig`
     2. Resolves references to previous steps
     3. Builds prompt with context if needed
     4. Routes to appropriate executor (stub/mock/real)
     5. Stores output for next steps

3. **Helper Functions**:
   - `extract_text_from_output()`: Gets text from CanonicalDocument
   - `build_summary_prompt()`: Creates prompts based on summary type
   - `build_prompt_with_context()`: Appends previous output as context

### Frontend Architecture

**File**: `app/src/components/CheckpointEditor.tsx`

1. **Dynamic Step Selection** (lines 97-118):
   - Filters available previous steps based on mode (create/edit)
   - Prevents circular references in edit mode
   - Shows descriptive step names: "Step 2: Summarize Brief"

2. **Step Type UI**:
   - Ingest: Document path, format, privacy status
   - Summarize: Source step, model, summary type, custom instructions
   - Prompt: Use output from (optional), model, prompt text

3. **JSON Generation**:
   - Builds camelCase JSON for typed steps
   - Includes fallback fields for robustness
   - Validates required fields before submission

## Example Workflows

### Research Paper Analysis

```
Step 1: Ingest Document
  - Type: Ingest Document
  - File: research_paper.pdf
  - Format: PDF
  → Output: CanonicalDocument with extracted text

Step 2: Summarize
  - Type: Summarize
  - Source: Step 1
  - Summary Type: Academic
  → Output: "This paper presents methodology X, findings Y, conclusions Z..."

Step 3: Prompt (with context)
  - Type: Prompt
  - Use Output From: Step 2
  - Prompt: "What are the limitations of this methodology?"
  → Output: Analysis based on the summary
```

### Multi-Model Chain

```
Step 1: Ingest Document
  → Processes PDF

Step 2: Summarize (llama3.2)
  → Brief summary from llama3.2

Step 3: Prompt (gpt-4)
  → Critical analysis from gpt-4 using llama's summary
```

## Technical Details

### JSON Structure

**Ingest**:
```json
{
  "stepType": "ingest",
  "sourcePath": "/path/to/document.pdf",
  "format": "pdf",
  "privacyStatus": "public"
}
```

**Summarize**:
```json
{
  "stepType": "summarize",
  "sourceStep": 0,
  "model": "llama3.2:1b",
  "summaryType": "brief",
  "tokenBudget": 2000,
  "proofMode": "exact"
}
```

**Prompt**:
```json
{
  "stepType": "prompt",
  "model": "llama3.2:1b",
  "prompt": "Explain why that's funny.",
  "useOutputFrom": 0,
  "tokenBudget": 1500,
  "proofMode": "exact"
}
```

### Context Chaining Format

When a Prompt step uses output from a previous step:

```
{user's prompt}

--- Context from previous step ---
{previous step's output}
```

Example:
```
Explain why that's funny.

--- Context from previous step ---
A man walked into a library and asked, "Do you have any books on Pavlov's dogs and Schrödinger's cat?" The librarian replied, "It rings a bell, but I'm not sure if it's here or not."
```

## Current Status

### ✅ What Works

1. **All Three Step Types**: Ingest, Summarize, Prompt
2. **Chaining**: Output flows correctly between steps
3. **Model Support**: Stub, mock, and real LLMs
4. **UI**: Dynamic dropdowns, clear step names
5. **Validation**: Prevents invalid references
6. **Execution**: Multi-step workflows complete successfully

### 🔧 Known Limitations

1. **Stub Model Output**: Generates hex hash (for testing), not human-readable
2. **Static Step Count**: Dropdowns work but could show more metadata
3. **No Visual Flow**: Can't see dependency graph visually
4. **Debug Logs**: Still active (should be configurable)

### 📝 Not Yet Implemented

1. **Step reordering with auto-update** of references
2. **Visual workflow builder** (drag & drop)
3. **Output preview** before execution
4. **Step templates** (save/reuse common patterns)
5. **Batch processing** (same workflow, multiple files)

## Files Modified

### Backend
- `src-tauri/src/orchestrator.rs`:
  - StepConfig enum (lines 41-93)
  - Execution logic (lines 1604-1913)
  - Helper functions (lines 1931-2094)
- `src-tauri/src/api.rs`: Validation (lines 1028-1046, 2195-2216)
- `src-tauri/src/store/migrations/V12__typed_step_system.sql`: Database schema

### Frontend
- `app/src/components/CheckpointEditor.tsx`:
  - Interface (lines 27-35)
  - Filtering logic (lines 97-118)
  - Step type UI (lines 475-590)
  - JSON generation (lines 211-320)
- `app/src/components/EditorPanel.tsx`:
  - Passing existingSteps (lines 1684-1688)
- `app/src/components/CheckpointListItem.tsx`:
  - Step preview display (lines 42-76)

## For Users

### Creating a Chained Workflow

1. **Add Step 1 (Ingest)**:
   - Click "Add Checkpoint"
   - Select "Ingest Document"
   - Browse for your file
   - Save

2. **Add Step 2 (Summarize)**:
   - Click "Add Checkpoint"
   - Select "Summarize"
   - Choose "Source Step: Step 1: {your file name}"
   - Select summary type
   - Save

3. **Add Step 3 (Prompt)**:
   - Click "Add Checkpoint"
   - Select "Prompt (with optional context)"
   - Choose "Use Output From: Step 2: Summarize"
   - Enter your prompt
   - Save

4. **Execute**:
   - Click "Execute Full Run"
   - Watch the console for chaining debug info
   - Check Inspector for results

### Best Practices

- **Start with Ingest**: Most workflows begin with document processing
- **Use Real Models**: Stub model is for testing structure, not content
- **Explicit Prompts**: Reference "the text below" or "previous response"
- **Check Debug Logs**: Verify context is being added
- **Name Steps Clearly**: Helps with dropdown selection

## Next Steps

See `MVP_ROADMAP.md` for prioritized enhancements.
