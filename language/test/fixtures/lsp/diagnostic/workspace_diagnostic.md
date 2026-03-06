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

```lsp workspace_diagnostic
file=main.ds
range=0:21-0:22
severity=error
code=EP001
source=destack
message=parse error: unexpected ; in Expression
```

```lsp open_text main.ds
export const value = [|;|]
```

