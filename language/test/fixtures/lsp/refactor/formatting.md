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

```lsp format_document main.ds [0]
```

```lsp current_file main.ds [0]
function main() {
    return 1;
}
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

```lsp format_selection format_start format_end [0]
```

```lsp current_file main.ds [0]
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

```lsp on_type_formatting on_type [0]
}
```

```lsp current_file main.ds [0]
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

```lsp set_format_option tabSize [0]
2
```

```lsp set_format_option insertSpaces [0]
true
```

```lsp verify_format_options [0]
tab_size=2
insert_spaces=true
```

### Disable and re-enable formatting

Formatting can be disabled and re-enabled while still producing the expected final file.

```ds:main.ds
function main(){return 1;}
```

```lsp disable_formatting [0]
```

```lsp format_document main.ds [0]
```

```lsp current_file main.ds [0]
function main(){return 1;}
```

```lsp enable_formatting [1]
```

```lsp format_document main.ds [1]
```

```lsp current_file main.ds [1]
function main() {
    return 1;
}
```

## Formatting Sequences

### Reformat a second overlay after the first pass already succeeded
Document formatting should keep rewriting each successive overlay state exactly.

```ds:main.ds
function first(){return 1;}
```

```ds:main.ds[1]
function first(){return 1;} function second(){return 2;}
```

```lsp format_document main.ds [0]
```

```lsp current_file main.ds [0]
function first() {
    return 1;
}
```

```lsp format_document main.ds [1]
```

```lsp current_file main.ds [1]
function first() {
    return 1;
}
function second() {
    return 2;
}
```

### Preserve formatting after a reindex between two passes
Document formatting should stay deterministic across an indexing command.

```ds:main.ds
function main(){return 1;}
```

```ds:main.ds[1]
function main(){return 1;} function helper(){return 2;}
```

```lsp format_document main.ds [0]
```

```lsp current_file main.ds [0]
function main() {
    return 1;
}
```

```lsp execute_command destack.reindex [1]
```

```lsp format_document main.ds [1]
```

```lsp current_file main.ds [1]
function main() {
    return 1;
}
function helper() {
    return 2;
}
```

### Rewrite the same selected block after it grows
Selection formatting should keep targeting the marked region after the block expands.

```ds:main.ds
function untouched() { return 0; }

[|/*format_start*/function main(){return 1;}/*format_end*/|]
```

```ds:main.ds[1]
function untouched() { return 0; }

[|/*format_start*/function main(){ return 1; return 2;
}/*format_end*/|]
```

```lsp format_selection format_start format_end [0]
```

```lsp current_file main.ds [0]
function untouched() { return 0; }

function main() {
    return 1;
}

```

```lsp format_selection format_start format_end [1]
```

```lsp current_file main.ds [1]
function untouched() { return 0; }

function main() {
    return 1;
    return 2;
}

```

### Keep on-type formatting stable across two trigger edits
On-type formatting should stay exact after a second trigger-driven overlay change.

```ds:main.ds
function main(){return 1;}/*on_type*/
```

```ds:main.ds[1]
function main(){return 1;} function helper(){return 2;}/*on_type*/
```

```lsp on_type_formatting on_type [0]
}
```

```lsp current_file main.ds [0]
function main() {
    return 1;
}

```

```lsp on_type_formatting on_type [1]
}
```

```lsp current_file main.ds [1]
function main() {
    return 1;
}
 function helper(){return 2;}
```

### Keep disabled formatting inert across overlay churn
Disabled formatting should leave later overlays untouched until formatting is re-enabled.

```ds:main.ds
function main(){return 1;}
```

```ds:main.ds[1]
function main(){return 1;} function helper(){return 2;}
```

```ds:main.ds[2]
function main(){return 1;} function helper(){return 2;}
```

```lsp disable_formatting [0]
```

```lsp format_document main.ds [0]
```

```lsp current_file main.ds [0]
function main(){return 1;}
```

```lsp format_document main.ds [1]
```

```lsp current_file main.ds [1]
function main(){return 1;} function helper(){return 2;}
```

```lsp enable_formatting [2]
```

```lsp format_document main.ds [2]
```

```lsp current_file main.ds [2]
function main() {
    return 1;
}
function helper() {
    return 2;
}
```

### Preserve formatting options across command churn
Formatting options should survive a command and still drive the later formatting pass.

```ds:main.ds
function main(){return 1;}
```

```ds:main.ds[1]
function main(){return 1;}
```

```lsp set_format_option tabSize [0]
2
```

```lsp set_format_option insertSpaces [0]
true
```

```lsp verify_format_options [0]
tab_size=2
insert_spaces=true
```

```lsp execute_command destack.reindex [1]
```

```lsp verify_format_options [1]
tab_size=2
insert_spaces=true
```

```lsp format_document main.ds [1]
```

```lsp current_file main.ds [1]
function main() {
    return 1;
}
```

## Formatting Churn

### Reformat one file while a sibling library grows
Document formatting should still rewrite the current file exactly while a sibling library changes.

```ds:lib.ds
export const value = 1;
```

```ds:main.ds
function main(){return 1 + value;}
```

```ds:lib.ds[1]
export const value = 1; export const bonus = 2;
```

```ds:main.ds[1]
function main(){return 1 + value + bonus;}
```

```lsp format_document main.ds [0]
```

```lsp current_file main.ds [0]
function main() {
    return 1 + value;
}
```

```lsp format_document main.ds [1]
```

```lsp current_file main.ds [1]
function main() {
    return 1 + value + bonus;
}
```

### Preserve selection formatting after a helper grows
Selection formatting should stay exact while an unrelated helper file grows across steps.

```ds:helper.ds
export const helper = 1;
```

```ds:main.ds
function untouched() { return helper; }

[|/*format_start*/function main(){return helper;}/*format_end*/|]
```

```ds:helper.ds[1]
export const helper = 1; export const extra = 2;
```

```ds:main.ds[1]
function untouched() { return helper + extra; }

[|/*format_start*/function main(){return helper + extra;}/*format_end*/|]
```

```lsp format_selection format_start format_end [0]
```

```lsp current_file main.ds [0]
function untouched() { return helper; }

function main() {
    return helper;
}

```

```lsp format_selection format_start format_end [1]
```

```lsp current_file main.ds [1]
function untouched() {
    return helper + extra;
}

function main() {
    return helper + extra;
}

```

### Keep on-type formatting stable while a sibling file churns
On-type formatting should remain exact while a sibling file changes across steps.

```ds:helper.ds
export const helper = 1;
```

```ds:main.ds
function main(){return helper;}/*on_type*/
```

```ds:helper.ds[1]
export const helper = 1; export const extra = 2;
```

```ds:main.ds[1]
function main(){return helper + extra;}/*on_type*/
```

```lsp on_type_formatting on_type [0]
}
```

```lsp current_file main.ds [0]
function main() {
    return helper;
}

```

```lsp on_type_formatting on_type [1]
}
```

```lsp current_file main.ds [1]
function main() {
    return helper + extra;
}

```

### Disable and re-enable formatting during sibling churn
Formatting can stay disabled through one step and re-enable cleanly after a sibling file changes.

```ds:helper.ds
export const helper = 1;
```

```ds:main.ds
function main(){return helper;}
```

```ds:helper.ds[1]
export const helper = 1; export const extra = 2;
```

```ds:main.ds[1]
function main() {
   return helper + extra;
}
```

```lsp disable_formatting [0]
```

```lsp format_document main.ds [0]
```

```lsp current_file main.ds [0]
function main(){return helper;}
```

```lsp enable_formatting [1]
```

```lsp format_document main.ds [1]
```

```lsp current_file main.ds [1]
function main() {
    return helper + extra;
}
```
