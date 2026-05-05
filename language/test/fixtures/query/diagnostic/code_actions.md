# Code Actions

Tests for LSP code actions functionality.

## Basic Tests

### No code actions for valid code

Valid code without errors should have no code actions.

```ds
$0function greet(name: string): string {
    return "Hello, " + name;
}

const msg = greet("World");
```

No code actions expected for valid code.

```query code_actions $0
<none>
```

### No code actions for minimal valid file

Minimal valid files without diagnostics should not generate code actions.

```ds
$0const x = 42;
```

No code actions expected.

```query code_actions $0
<none>
```

### Auto-import quick fix for missing symbol

Missing symbols should offer auto-import code actions when an exported match exists.

```ds:lib.ds
export function greet(): void {}
```

```ds:main.ds
function main() {
    $0greet();
}
```

Auto-import fixes should insert an import statement.

```query code_actions $0
[0] title=Import greet from "./lib" kind=quick_fix preferred=true diag=<none> edits=main.ds:1:1-1:1=>"import { greet } from "./lib";\n"
```

### Auto-import type-only quick fix for pure type exports in type position

Missing pure type references should offer type-only import code actions.

```ds:lib.ds
export type Widget = {
    value: string,
};
```

```ds:main.ds
type Alias = $0Widget;
```

Auto-import fixes should insert a type-only import statement.

```query code_actions $0
[0] title=Import Widget from "./lib" kind=quick_fix preferred=true diag=<none> edits=main.ds:1:1-1:1=>"import type { Widget } from "./lib";\n"
```

### Auto-import type-only quick fix for pure type exports in type position

Missing Destack pure type references should offer type-only import code actions.

```ds:lib.ds
export type Widget = {
    value: string,
};
```

```ds:main.ds
export {}

type Alias = $0Widget
```

Auto-import fixes should insert a type-only import statement.

```query code_actions $0
[0] title=Import Widget from "./lib" kind=quick_fix preferred=true diag=<none> edits=main.ds:1:1-1:1=>"import type { Widget } from "./lib";\n"
```

### Auto-import type-only quick fix for type-value exports in type position

Missing Destack type-value references should still use type-only imports in type position.

```ds:lib.ds
export class WobbleWidget {}
```

```ds:main.ds
export {}

type Alias = $0WobbleWidget
```

Auto-import fixes should use `import type` so the import stays type-only.

```query code_actions $0
[0] title=Import WobbleWidget from "./lib" kind=quick_fix preferred=true diag=<none> edits=main.ds:1:1-1:1=>"import type { WobbleWidget } from "./lib";\n"
```

### Auto-import value quick fix for .ds type-value exports in type position

Missing Destack type-value references should keep full imports in type position.

```ds:lib.ds
export struct Widget {}
```

```ds:main.ds
type Alias = $0Widget;
```

Auto-import fixes should keep the value import so the symbol stays dual-space.

```query code_actions $0
[0] title=Import Widget from "./lib" kind=quick_fix preferred=true diag=<none> edits=main.ds:1:1-1:1=>"import { Widget } from "./lib";\n"
```

### Rank same-name auto-import fixes by proximity

Missing symbols with multiple import candidates should rank nearer modules first.

```ds:near/format.ds
export function formatWidget(): void {}
```

```ds:far/deeper/format.ds
export function formatWidget(): void {}
```

```ds:main.ds
function main() {
    $0formatWidget();
}
```

```query code_actions $0
[0] title=Import formatWidget from "./near/format" kind=quick_fix preferred=true diag=<none> edits=main.ds:1:1-1:1=>"import { formatWidget } from "./near/format";\n"
[1] title=Import formatWidget from "./far/deeper/format" kind=quick_fix preferred=false diag=<none> edits=main.ds:1:1-1:1=>"import { formatWidget } from "./far/deeper/format";\n"
```

### Organize imports for unsorted imports

Organize imports should reorder top-level imports by module path.

```ds
$0import { zed } from "./zeta.ds";
import { alpha } from "./alpha.ds";

const value = alpha + zed;
```

```query code_actions $0
[0] title=Organize Imports kind=source_organize_imports preferred=false diag=<none> edits=main.ds:1:1-2:36=>"import { alpha } from "./alpha.ds";\nimport { zed } from "./zeta.ds";"
```

### No organize imports when already sorted

Organize imports should not appear when imports are already ordered.

```ds
$0import { alpha } from "./alpha.ds";
import { zed } from "./zeta.ds";

const value = alpha + zed;
```

```query code_actions $0
<none>
```

### Filter to quick fixes with only directive

Code action queries should support `only:` filters for quick fixes.

```ds:lib.ds
export function greet(): void {}
```

```ds:main.ds
import { zed } from "./zeta.ds";
import { alpha } from "./alpha.ds";

function main() {
    $0greet();
    return alpha + zed;
}
```

Only quick fixes should be returned.

```query code_actions $0
only: quick_fix
[0] title=Import greet from "./lib" kind=quick_fix preferred=true diag=<none> edits=main.ds:1:1-1:1=>"import { greet } from "./lib";\n"
```

### Filter to source organize imports with only directive

Code action queries should support `only:` filters for source organize imports.

```ds
$0import { zed } from "./zeta.ds";
import { alpha } from "./alpha.ds";

const value = alpha + zed;
```

Only organize imports actions should be returned.

```query code_actions $0
only: source.organizeImports
[0] title=Organize Imports kind=source_organize_imports preferred=false diag=<none> edits=main.ds:1:1-2:36=>"import { alpha } from "./alpha.ds";\nimport { zed } from "./zeta.ds";"
```

### Filter to multiple kinds with only directive

Code action queries should support multiple kinds in the `only:` directive.

```ds:lib.ds
export function greet(): void {}
```

```ds:main.ds
import { zed } from "./zeta.ds";
import { alpha } from "./alpha.ds";

function main() {
    $0greet();
    return alpha + zed;
}
```

Quick fixes and organize imports should both be returned.

```query code_actions $0
only: quick_fix, source_organize_imports
[0] title=Import greet from "./lib" kind=quick_fix preferred=true diag=<none> edits=main.ds:1:1-1:1=>"import { greet } from "./lib";\n"
[1] title=Organize Imports kind=source_organize_imports preferred=false diag=<none> edits=main.ds:1:1-2:36=>"import { alpha } from "./alpha.ds";\nimport { zed } from "./zeta.ds";"
```

### Extract constant refactor for selected expressions

Selected expression ranges should offer extract constant refactors.

```ds
function main(a: int32, b: int32): int32 {
    return a + b;
    //       ^^^^^ selection
}
```

```query code_actions selection
only: refactor_extract
top: 1
[0] title=Extract constant kind=refactor_extract preferred=false diag=<none> edits=main.ds:2:1-2:1=>"    const extracted = a + b;\n", main.ds:2:12-2:17=>"extracted"
```

## Damaged Syntax

### Keep auto-import fixes after malformed call statements

Code actions should still offer later valid auto-import fixes after one malformed call statement.

```ds:lib.ds
export function greet(): void {}
```

```ds:main.ds
broken(,

function main() {
    $0greet();
}
```

```query code_actions $0
[0] title=Import greet from "./lib" kind=quick_fix preferred=true diag=<none> edits=main.ds:1:1-1:1=>"import { greet } from "./lib";\n"
```

### Keep auto-import fixes after bare new recovery statements

Code actions should still offer later valid auto-import fixes after one bare `new` recovery statement.

```ds:lib.ds
export function greet(): void {}
```

```ds:main.ds
new

function main() {
    $0greet();
}
```

```query code_actions $0
[0] title=Import greet from "./lib" kind=quick_fix preferred=true diag=<none> edits=main.ds:1:1-1:1=>"import { greet } from "./lib";\n"
```

### Keep auto-import fixes after throw recovery statements

Code actions should still offer later valid auto-import fixes after one recovered `throw` statement.

```ds:lib.ds
export function greet(): void {}
```

```ds:main.ds
throw

function main() {
    $0greet();
}
```

```query code_actions $0
[0] title=Import greet from "./lib" kind=quick_fix preferred=true diag=<none> edits=main.ds:1:1-1:1=>"import { greet } from "./lib";\n"
```

### Keep auto-import fixes after yield star recovery statements

Code actions should still offer later valid auto-import fixes after one recovered `yield*` statement.

```ds:lib.ds
export function greet(): void {}
```

```ds:main.ds
function* broken() {
    yield*
    const value = 1;
}

function main() {
    $0greet();
}
```

```query code_actions $0
[0] title=Import greet from "./lib" kind=quick_fix preferred=true diag=<none> edits=main.ds:1:1-1:1=>"import { greet } from "./lib";\n"
```
