# Editing

## Overlay Editing

### Replace a selected span

Editor replacement should roundtrip the selected span into the expected file contents.

```ds:main.ds
const message = "he/*select_start*/l/*after_edit*/lo/*select_end*/"/*edit*/;
```

```lsp scenario edit-roundtrip
```

```lsp current_file
const message = "hey";
```

### Replace the whole file through selection

Selecting the whole file should allow one exact full-document replacement.

```ds:main.ds
const first = 1;
```

```lsp scenario edit-select-all-replace
```

```lsp current_file
const first = 1;
```

### Replace text and insert lines

The editor surface should support mixed replacement and line insertion in one flow.

```ds:main.ds
const /*replace_name*/alpha = /*replace_number*/1;
```

```lsp scenario edit-replace-and-insert-lines
```

```lsp current_file
const gamma = 3;
const delta = 4;
const epsilon = 5;
```

### Replace one marked range

The editor surface should replace a marked range without disturbing the surrounding text.

```ds:main.ds
export const [|beta|] = 2;
```

```lsp scenario edit-select-range-replace
```

```lsp current_file
export const omega = 2;
```

### Paste, delete, and return to the beginning

The editor surface should preserve the expected text across paste, delete, and caret-reset operations.

```ds:main.ds
const body = "/*caret*/core";
```

```lsp scenario edit-paste-delete-and-bof
```

```lsp current_file
// header
const body = "more";
const tail = 1;
```

## Line Editing

### Delete one line

Deleting one line should preserve the remaining file shape exactly.

```ds:main.ds
const alpha = 1;
const beta = 2;
const gamma = 3;
```

```lsp scenario edit-delete-line
```

```lsp current_file
const alpha = 1;
const gamma = 3;
```

### Delete an inclusive line range

Deleting a line range should remove the requested span and keep the remaining text exact.

```ds:main.ds
const alpha = 1;
const beta = 2;
const gamma = 3;
const delta = 4;
```

```lsp scenario edit-delete-line-range
```

```lsp current_file
const alpha = 1;
const delta = 4;
```

### Replace one line

Replacing one line should preserve the surrounding newline shape.

```ds:main.ds
const alpha = 1;
const beta = 2;
const gamma = 3;
```

```lsp scenario edit-replace-line
```

```lsp current_file
const alpha = 1;
const beta = 20;
const gamma = 3;
```

## Multi-File Flows

### Repeated edits stay responsive across files

Definitions and diagnostics should stay responsive across repeated edits in multiple files.

```ds:lib.ds
export const /*def*/value: number = 1;
```

```ds:main.ds
import { value } from "./lib.ds";
const output: number = /*use*/value + 1;
export { output };
```

```lsp scenario multifile-edit-responsiveness
```

## Caret State

### Report the current line indentation

The editor surface should report the indentation of the current line exactly.

```ds:main.ds
function main() {
    /*indent*/return 1;
}
```

```lsp scenario indentation-current-line
```
