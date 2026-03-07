# Workspace Diagnostic

## Full Reports

### Whole workspace snapshot

Workspace diagnostics should return the full normalized report for the current workspace state.

```ds:main.ds
export const value = [|;|]
```

```lsp workspace_diagnostic
file=main.ds
range=0:21-0:22
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

## Partial Reports

### Partial workspace snapshot

Workspace diagnostics should return the normalized partial report when the server streams results.

```ds:main.ds
export const value = [|;|]
```

```lsp workspace_diagnostic_partial
file=main.ds
range=0:21-0:22
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

## Open Overlays

### Overlay text participates in workspace diagnostics

Workspace diagnostics should include the open overlay text instead of the stale on-disk file.

```ds:main.ds
export const value: number = 1;
```

```ds:main.ds[1]
export const value = [|;|]
```

```lsp close main.ds [1]
```

```lsp open_text main.ds [1]
export const value = [|;|]
```

```lsp workspace_diagnostic [1]
file=main.ds
range=0:21-0:22
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

## File Creation And Removal

### Create one broken file and then delete it again
Workspace diagnostics should report a newly created broken file and clear once that file disappears.

```ds:main.ds
export const value = 1;
```

```ds:generated.ds[1]
export const broken = ;
```

```lsp workspace_diagnostic [0]
```

```lsp workspace_diagnostic [1]
file=generated.ds
range=0:22-0:23
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp delete_file generated.ds [2]
```

```lsp workspace_diagnostic [2]
```

## Workspace diagnostics

### Report two broken files in one workspace snapshot
Workspace diagnostics should include both files after both overlays become invalid.

```ds:a.ds
export const a = 1;
```

```ds:b.ds
export const b = 2;
```

```ds:a.ds[1]
export const a = ;
```

```ds:b.ds[1]
export const b = ;
```

```lsp workspace_diagnostic [1]
file=a.ds
range=0:17-0:18
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression

file=b.ds
range=0:17-0:18
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Fix one file and leave the other broken
Workspace diagnostics should retain only the remaining broken file after one fix lands.

```ds:a.ds
export const a = ;
```

```ds:b.ds
export const b = ;
```

```ds:a.ds[1]
export const a = 1;
```

```ds:b.ds[1]
export const b = ;
```

```lsp workspace_diagnostic [0]
file=a.ds
range=0:17-0:18
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression

file=b.ds
range=0:17-0:18
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp workspace_diagnostic [1]
file=b.ds
range=0:17-0:18
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Clear both workspace errors after both fixes land
Workspace diagnostics should return to an empty report after both broken overlays are fixed.

```ds:a.ds
export const a = ;
```

```ds:b.ds
export const b = ;
```

```ds:a.ds[1]
export const a = 1;
```

```ds:b.ds[1]
export const b = 2;
```

```lsp workspace_diagnostic [0]
file=a.ds
range=0:17-0:18
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression

file=b.ds
range=0:17-0:18
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp workspace_diagnostic [1]
```

### Preserve a saved workspace error after close
Workspace diagnostics should keep the saved broken file after the file closes.

```ds:a.ds
export const a = 1;
```

```ds:b.ds
export const b = 2;
```

```ds:a.ds[1]
export const a = ;
```

```ds:b.ds[1]
export const b = 2;
```

```lsp save a.ds [1]
```

```lsp close a.ds [1]
```

```lsp workspace_diagnostic [1]
file=a.ds
range=0:17-0:18
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Keep workspace diagnostics stable after reindexing a broken overlay
Workspace diagnostics should still report the same broken overlay after a reindex command.

```ds:main.ds
export const value = ;
```

```lsp execute_command destack.reindex [0]
```

```lsp workspace_diagnostic [0]
file=main.ds
range=0:21-0:22
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

## Staggered Fixes

### Fix three broken files one at a time
Workspace diagnostics should shrink as each broken file is fixed.

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
const b = ;
```

```ds:c.ds[1]
const c = ;
```

```ds:b.ds[2]
const b = 2;
```

```ds:a.ds[2]
const a = 1;
```

```ds:c.ds[2]
const c = ;
```

```ds:c.ds[3]
const c = 3;
```

```lsp workspace_diagnostic [0]
file=a.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression

file=b.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression

file=c.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp workspace_diagnostic [1]
file=b.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression

file=c.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp workspace_diagnostic [2]
file=c.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp workspace_diagnostic [3]
```

### Grow and shrink workspace diagnostics across four overlay states
Workspace diagnostics should follow the full overlay sequence without skipping intermediate states.

```ds:a.ds
const a = 1;
```

```ds:b.ds
const b = 2;
```

```ds:c.ds
const c = 3;
```

```ds:a.ds[1]
const a = ;
```

```ds:a.ds[2]
const a = ;
```

```ds:b.ds[2]
const b = ;
```

```ds:a.ds[3]
const a = 1;
```

```ds:b.ds[3]
const b = ;
```

```ds:c.ds[3]
const c = ;
```

```ds:a.ds[4]
const a = 1;
```

```ds:b.ds[4]
const b = 2;
```

```ds:c.ds[4]
const c = 3;
```

```lsp workspace_diagnostic [0]
```

```lsp workspace_diagnostic [1]
file=a.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp workspace_diagnostic [2]
file=a.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression

file=b.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp workspace_diagnostic [3]
file=b.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression

file=c.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp workspace_diagnostic [4]
```

### Break three clean files one at a time
Workspace diagnostics should grow as more files become broken.

```ds:a.ds
const a = 1;
```

```ds:b.ds
const b = 2;
```

```ds:c.ds
const c = 3;
```

```ds:a.ds[1]
const a = ;
```

```ds:b.ds[1]
const b = 2;
```

```ds:c.ds[1]
const c = 3;
```

```ds:b.ds[2]
const b = ;
```

```ds:a.ds[2]
const a = ;
```

```ds:c.ds[2]
const c = 3;
```

```ds:c.ds[3]
const c = ;
```

```lsp workspace_diagnostic [0]
```

```lsp workspace_diagnostic [1]
file=a.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp workspace_diagnostic [2]
file=a.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression

file=b.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp workspace_diagnostic [3]
file=a.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression

file=b.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression

file=c.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

## Command Churn

### Reindex while three files stay broken
Reindex should not change the exact workspace diagnostic set for broken files.

```ds:a.ds
const a = ;
```

```ds:b.ds
const b = ;
```

```ds:c.ds
const c = ;
```

```lsp execute_command destack.reindex [0]
```

```lsp workspace_diagnostic [0]
file=a.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression

file=b.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression

file=c.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Rescan while one file fixes and another breaks
Rescan should reflect the new mixed diagnostic set exactly.

```ds:a.ds
const a = ;
```

```ds:b.ds
const b = 1;
```

```ds:a.ds[1]
const a = 1;
```

```ds:b.ds[1]
const b = ;
```

```lsp execute_command destack.rescan [1]
```

```lsp workspace_diagnostic [1]
file=b.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Clear cache while one saved fix and one unsaved break coexist
Clear cache should preserve the exact mixed diagnostic state.

```ds:a.ds
const a = ;
```

```ds:b.ds
const b = 1;
```

```ds:a.ds[1]
const a = 1;
```

```ds:b.ds[1]
const b = ;
```

```lsp save a.ds [1]
```

```lsp execute_command destack.clearCache [1]
```

```lsp workspace_diagnostic [1]
file=b.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

## Mixed Save Sequences

### Fix one file, break another, and save both states
Workspace diagnostics should reflect one saved fix and one newly broken sibling exactly.

```ds:a.ds
const a = ;
```

```ds:b.ds
const b = 1;
```

```ds:c.ds
const c = 1;
```

```ds:a.ds[1]
const a = 1;
```

```ds:b.ds[1]
const b = ;
```

```ds:c.ds[1]
const c = 1;
```

```lsp save a.ds [1]
```

```lsp workspace_diagnostic [1]
file=b.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Save two broken files while a third overlay fixes cleanly
Workspace diagnostics should keep both saved broken files after the clean overlay stays fixed.

```ds:a.ds
const a = ;
```

```ds:b.ds
const b = ;
```

```ds:c.ds
const c = ;
```

```ds:c.ds[1]
const c = 3;
```

```ds:a.ds[1]
const a = ;
```

```ds:b.ds[1]
const b = ;
```

```lsp save a.ds [1]
```

```lsp save b.ds [1]
```

```lsp workspace_diagnostic [1]
file=a.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression

file=b.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

## Close And Reopen

### Close one broken overlay while a saved sibling stays broken
Closing one overlay should drop only that file while preserving the saved broken sibling.

```ds:a.ds
const a = ;
```

```ds:b.ds
const b = ;
```

```ds:a.ds[1]
const a = ;
```

```ds:b.ds[1]
const b = ;
```

```lsp save b.ds [1]
```

```lsp close a.ds [1]
```

```lsp workspace_diagnostic [1]
file=a.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression

file=b.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

### Reopen a fixed overlay after a sibling stays broken
Reopening a fixed overlay should keep diagnostics on the still broken sibling only.

```ds:a.ds
const a = ;
```

```ds:b.ds
const b = ;
```

```ds:a.ds[1]
const a = 1;
```

```ds:b.ds[1]
const b = ;
```

```lsp close a.ds [1]
```

```lsp open_text a.ds [1]
const a = 1;
```

```lsp workspace_diagnostic [1]
file=b.ds
range=0:10-0:11
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```
