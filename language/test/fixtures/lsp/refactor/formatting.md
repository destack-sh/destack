# Formatting

## Whole Document

### Rewrite one unformatted file

Document formatting should rewrite the full file into the expected layout.

```ds:main.ds
function main(){return 1;}
```

```lsp document_formatting
function main() {
    return 1;
}
```

### Preserve an already formatted file

Document formatting should leave an already formatted file unchanged.

```ds:main.ds
function main() {
    return 1;
}
```

```lsp scenario document-formatting-changes-nothing
```

## Range Formatting

### Format one function block

Range formatting should rewrite only the selected block and preserve the surrounding text.

```ds:main.ds
function untouched(){return 0;}

[|function main(){return 1;}|]
```

```lsp range_formatting
function untouched(){return 0;}

function main() {
    return 1;
}

```

### Format one marked selection

Selection formatting should rewrite the exact marked selection span.

```ds:main.ds
function untouched() { return 0; }

[|/*format_start*/function main(){return 1;}/*format_end*/|]
```

```lsp scenario format-selection-markers
```

```lsp range_formatting
function untouched() { return 0; }

function main() {
    return 1;
}

```

## On-Type Formatting

### Format after typing a trigger character

On-type formatting should rewrite the document after the trigger character is typed.

```ds:main.ds
function main(){return 1;}/*on_type*/
```

```lsp scenario on-type-formatting-brace
```

```lsp current_file
function main() {
    return 1;
}

```

## Option Flows

### Roundtrip formatting options

Formatting options should roundtrip through the harness without losing their values.

```ds:main.ds
function main() {
    return 1;
}
```

```lsp scenario format-option-roundtrip
```

### Disable and re-enable formatting

Formatting can be disabled and re-enabled while still producing the expected final file.

```ds:main.ds
function main(){return 1;}
```

```lsp scenario format-disable-enable-roundtrip
```

```lsp current_file
function main() {
    return 1;
}
```
