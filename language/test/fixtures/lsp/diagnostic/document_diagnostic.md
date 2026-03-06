# Document Diagnostic

## Overlay Lifecycle

### Opening an overlay changes diagnostics

Document diagnostics should reflect the overlay text after the file is opened in the editor.

```ds:main.ds
export const /*diag*/value = [|;|]
```

```lsp scenario document-diagnostic-open-overlay
```

### Editing an overlay changes diagnostics

Document diagnostics should update after an unsaved overlay edit.

```ds:main.ds
export const /*change*/value = 1;
```

```lsp scenario document-diagnostic-change-overlay
```

### Saving persists overlay diagnostics

Document diagnostics should keep the saved content after the overlay is written to disk.

```ds:main.ds
export const /*save*/value = 1;
```

```lsp scenario document-diagnostic-save-persists-overlay
```

### Closing reverts to disk diagnostics

Document diagnostics should revert to the on-disk content after the overlay closes.

```ds:main.ds
export const /*close*/value = 1;
```

```lsp scenario document-diagnostic-close-reverts-overlay
```

## Baselines

### Clean files produce no diagnostics

Document diagnostics should stay empty for a valid file.

```ds:main.ds
export const value = 1;
```

```lsp scenario no-errors-clean-workspace
```

### Marker windows locate one diagnostic

Document diagnostics should line up with the marker window around the expected error span.

```ds:main.ds
const value = /*error_start*//*error_before*/;/*error_after*//*error_end*/
```

```lsp scenario diagnostic-marker-windows
```

