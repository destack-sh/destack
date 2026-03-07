# Editing

## Overlay Editing

### Replace a selected span

Editor replacement should roundtrip the selected span into the expected file contents.

```ds:main.ds
const message = "he/*select_start*/l/*after_edit*/lo/*select_end*/"/*edit*/;
```

```lsp select_markers select_start select_end [0]
```

```lsp replace_selection [0]
y
```

```lsp current_file main.ds [0]
const message = "hey";
```

### Replace the whole file through selection

Selecting the whole file should allow one exact full-document replacement.

```ds:main.ds
const first = 1;
```

```lsp select_all main.ds [0]
```

```lsp replace_selection [0]
const first = 1;

```

```lsp current_file main.ds [0]
const first = 1;
```

### Replace text and insert lines

The editor surface should support mixed replacement and line insertion in one flow.

```ds:main.ds
const /*replace_name*/alpha = /*replace_number*/1;
```

```lsp select_all main.ds [0]
```

```lsp replace_selection [0]
const gamma = 3;
const delta = 4;

```

```lsp paste [0]
const epsilon = 5;

```

```lsp current_file main.ds [0]
const gamma = 3;
const delta = 4;
const epsilon = 5;
```

### Replace one marked range

The editor surface should replace a marked range without disturbing the surrounding text.

```ds:main.ds
export const /*beta_start*/[|beta|]/*beta_end*/ = 2;
```

```lsp select_markers beta_start beta_end [0]
```

```lsp replace_selection [0]
omega
```

```lsp current_file main.ds [0]
export const omega = 2;
```

### Paste, delete, and return to the beginning

The editor surface should preserve the expected text across paste, delete, and caret-reset operations.

```ds:main.ds
const body = "/*caret*/core";
```

```lsp go_to_marker caret [0]
```

```lsp paste [0]
more
```

```lsp delete_at_caret 4 [0]
```

```lsp go_to_bof [0]
```

```lsp paste [0]
// header

```

```lsp current_file main.ds [0]
// header
const body = "more";
```

```lsp move_right 21 [0]
```

```lsp paste [0]
const tail = 1;

```

```lsp current_file main.ds [0]
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

```lsp delete_line 1 [0]
```

```lsp current_file main.ds [0]
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

```lsp delete_line_range 1 2 [0]
```

```lsp current_file main.ds [0]
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

```lsp replace_line 1 [0]
const beta = 20;
```

```lsp current_file main.ds [0]
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

```lsp definition use [0]
def
```

```ds:lib.ds[1]
export const /*def*/value: number = 2;
```

```ds:main.ds[1]
import { value } from "./lib.ds";
const output: number = /*use*/value + 2;
export { output };
```

```lsp definition use [1]
def
```

```lsp document_diagnostic main.ds [1]
```

## Caret State

### Report the current line indentation

The editor surface should report the indentation of the current line exactly.

```ds:main.ds
function main() {
    /*indent*/return 1;
}
```

```lsp indentation indent [0]
4
```
