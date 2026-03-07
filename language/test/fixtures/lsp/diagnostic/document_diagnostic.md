# Document Diagnostic

## Overlay Lifecycle

### Opening an overlay changes diagnostics

Document diagnostics should reflect the overlay text after the file is opened in the editor.

```ds:main.ds
export const value = 1;
```

```lsp close main.ds [1]
```

```lsp open_text main.ds [1]
export const /*diag*/value = [|;|]
```

```lsp document_diagnostic main.ds [1]
file=main.ds
range=0:21-0:22
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Editing an overlay changes diagnostics

Document diagnostics should update after an unsaved overlay edit.

```ds:main.ds
export const value = 1;
```

```ds:main.ds[1]
export const /*change*/value = ;
```

```lsp document_diagnostic main.ds [1]
file=main.ds
range=0:21-0:22
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Saving persists overlay diagnostics

Document diagnostics should keep the saved content after the overlay is written to disk.

```ds:main.ds
export const value = 1;
```

```ds:main.ds[1]
export const /*save*/value = ;
```

```lsp save main.ds [1]
```

```lsp document_diagnostic main.ds [1]
file=main.ds
range=0:21-0:22
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Closing reverts to disk diagnostics

Document diagnostics should revert to the on-disk content after the overlay closes.

```ds:main.ds
export const value = 1;
```

```ds:main.ds[1]
export const /*close*/value = ;
```

```lsp close main.ds [1]
```

```lsp document_diagnostic main.ds [1]
```

## Baselines

### Clean files produce no diagnostics

Document diagnostics should stay empty for a valid file.

```ds:main.ds
export const value = 1;
```

```lsp no_errors main.ds [0]
```

### Marker windows locate one diagnostic

Document diagnostics should line up with the marker window around the expected error span.

```ds:main.ds
const value = /*error_start*//*error_before*/;/*error_after*//*error_end*/
```

```lsp diagnostic_marker_windows error_start error_end [0]
```

## Document diagnostics

### Clear a document error after fixing the overlay
Document diagnostics should become empty after the overlay fixes the parse error.

```ds:main.ds
export const value = ;
```

```ds:main.ds[1]
export const value = 1;
```

```lsp document_diagnostic main.ds [0]
file=main.ds
range=0:21-0:22
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp document_diagnostic main.ds [1]
```

### Go from clean to broken and back to clean in one session
Document diagnostics should track both the regression and the later fix.

```ds:main.ds
export const value = 1;
```

```ds:main.ds[1]
export const value = ;
```

```ds:main.ds[2]
export const value = 2;
```

```lsp document_diagnostic main.ds [0]
```

```lsp document_diagnostic main.ds [1]
file=main.ds
range=0:21-0:22
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp document_diagnostic main.ds [2]
```

### Preserve a saved error after close and reopen
Document diagnostics should keep the saved on-disk error after the file is closed and reopened.

```ds:main.ds
export const value = 1;
```

```ds:main.ds[1]
export const value = ;
```

```lsp save main.ds [1]
```

```lsp close main.ds [1]
```

```lsp open_text main.ds [1]
export const value = ;
```

```lsp document_diagnostic main.ds [1]
file=main.ds
range=0:21-0:22
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Drop an unsaved error after the overlay closes
Document diagnostics should return to the clean disk state when the unsaved overlay is closed.

```ds:main.ds
export const value = 1;
```

```ds:main.ds[1]
export const value = ;
```

```lsp close main.ds [1]
```

```lsp document_diagnostic main.ds [1]
```

### Save a fix and keep the document clean after close
Document diagnostics should stay empty after a clean fix is saved and the file closes.

```ds:main.ds
export const value = ;
```

```ds:main.ds[1]
export const value = 1;
```

```lsp save main.ds [1]
```

```lsp close main.ds [1]
```

```lsp document_diagnostic main.ds [1]
```

## Per-File Tracking

### Track one file while a second file fixes
Document diagnostics should stay exact for the selected file while another file changes.

```ds:a.ds
const a = ;
```

```ds:b.ds
const b = ;
```

```ds:b.ds[1]
const b = 2;
```

```lsp document_diagnostic a.ds [0]
file=a.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp document_diagnostic a.ds [1]
file=a.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Clear one file while another remains broken through save and close
Document diagnostics should stay empty for the fixed file after save and close.

```ds:a.ds
const a = ;
```

```ds:b.ds
const b = ;
```

```ds:a.ds[1]
const a = 1;
```

```lsp save a.ds [1]
```

```lsp close a.ds [1]
```

```lsp document_diagnostic a.ds [1]
```

```lsp workspace_diagnostic [1]
file=b.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Preserve one saved broken file while another closes cleanly
Workspace diagnostics should keep only the saved broken file after the clean file closes.

```ds:a.ds
const a = ;
```

```ds:b.ds
const b = 1;
```

```ds:b.ds[1]
const b = 2;
```

```lsp save a.ds [0]
```

```lsp close b.ds [1]
```

```lsp workspace_diagnostic [1]
file=a.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Fix the last broken file after two earlier saves
Workspace diagnostics should go empty when the final broken file is fixed last.

```ds:a.ds
const a = ;
```

```ds:b.ds
const b = ;
```

```ds:c.ds
const c = ;
```

```ds:a.ds[1]
const a = 1;
```

```ds:b.ds[1]
const b = 2;
```

```ds:c.ds[1]
const c = ;
```

```ds:c.ds[2]
const c = 3;
```

```lsp save a.ds [1]
```

```lsp save b.ds [1]
```

```lsp workspace_diagnostic [1]
file=c.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp workspace_diagnostic [2]
```

## Per-File Tracking

### Keep one document clean while two siblings churn
Document diagnostics should stay empty for the selected clean file while two sibling files churn around it.

```ds:focus.ds
const focus = 1;
```

```ds:a.ds
const a = ;
```

```ds:b.ds
const b = 2;
```

```ds:a.ds[1]
const a = 1;
```

```ds:b.ds[1]
const b = ;
```

```lsp document_diagnostic focus.ds [0]
```

```lsp document_diagnostic focus.ds [1]
```
