# Completion

## Member Access

### Partial field names

Completion should return the matching member fields for a partial dotted access.

```ds:main.ds
struct Point {
    xa: int32
    xb: int32
    y: int32
}

function main() {
    const p = Point { xa: 1, xb: 2, y: 3 };
    p.x/*completion*/
}
```

```lsp completion_item
xa|field
xb|field
```

## Resolve

### Resolve completion documentation on demand

Completion resolve should materialize documentation for the selected item.

```ds:lib.ds
/// Paint one color.
export function paint(color: string): void {}
```

```ds:main.ds
import { paint } from "./lib.ds";

pa/*completion*/
```

```lsp completion_resolve_label
paint
```

```lsp completion_resolve_documentation
Paint one color.
```

## Completion churn

### Grow member completions after adding another matching field
Completion should include newly added members that match the same prefix after the overlay edit.

```ds:main.ds
struct Point {
   xa: int32
   y: int32
}
function main() {
   const point = Point { xa: 1, y: 2 };
   point.x/*completion*/
}
```

```ds:main.ds[1]
struct Point {
   xa: int32
   xb: int32
   y: int32
}
function main() {
   const point = Point { xa: 1, xb: 2, y: 3 };
   point.x/*completion*/
}
```

```lsp completion completion [0]
xa|field
```

```lsp completion completion [1]
xa|field
xb|field
```

### Shrink member completions after removing one matching field
Completion should drop deleted members from the exact completion set.

```ds:main.ds
struct Point {
   xa: int32
   xb: int32
}
function main() {
   const point = Point { xa: 1, xb: 2 };
   point.x/*completion*/
}
```

```ds:main.ds[1]
struct Point {
   xa: int32
}
function main() {
   const point = Point { xa: 1 };
   point.x/*completion*/
}
```

```lsp completion completion [0]
xa|field
xb|field
```

```lsp completion completion [1]
xa|field
```

### Resolve completion documentation across three prefixes
Completion resolve should follow the current prefix through multiple overlay edits.

```ds:lib.ds
/// Paint one color.
export function paint(color: string): void {}
/// Count one value.
export function count(value: int32): void {}
/// Print one line.
export function print(line: string): void {}
```

```ds:main.ds
import { paint, count, print } from "./lib.ds";
pa/*completion*/
```

```ds:lib.ds[1]
/// Paint one color.
export function paint(color: string): void {}
/// Count one value.
export function count(value: int32): void {}
/// Print one line.
export function print(line: string): void {}
```

```ds:main.ds[1]
import { paint, count, print } from "./lib.ds";
co/*completion*/
```

```ds:lib.ds[2]
/// Paint one color.
export function paint(color: string): void {}
/// Count one value.
export function count(value: int32): void {}
/// Print one line.
export function print(line: string): void {}
```

```ds:main.ds[2]
import { paint, count, print } from "./lib.ds";
pr/*completion*/
```

```lsp completion_resolve completion [0]
label=paint
documentation=Paint one color.
```

```lsp completion_resolve completion [1]
label=count
documentation=Count one value.
```

```lsp completion_resolve completion [2]
label=print
documentation=Print one line.
```

### Resolve completion documentation across four states with added churn
Completion resolve should stay exact as the active prefix shifts through multiple overlay states.

```ds:lib.ds
/// Paint one color.
export function paint(color: string): void {}
/// Count one value.
export function count(value: int32): void {}
/// Print one line.
export function print(line: string): void {}
/// Play one tone.
export function play(tone: string): void {}
```

```ds:main.ds
import { paint, count, print, play } from "./lib.ds";
pa/*completion*/
```

```ds:main.ds[1]
import { paint, count, print, play } from "./lib.ds";
co/*completion*/
```

```ds:lib.ds[2]
/// Paint one color.
export function paint(color: string): void {}
/// Count one value.
export function count(value: int32): void {}
/// Print one line.
export function print(line: string): void {}
/// Play one tone.
export function play(tone: string): void {}
export const helper = 1;
```

```ds:main.ds[2]
import { paint, count, print, play } from "./lib.ds";
pr/*completion*/
```

```ds:main.ds[3]
import { paint, count, print, play } from "./lib.ds";
pl/*completion*/
```

```lsp completion_resolve completion [0]
label=paint
documentation=Paint one color.
```

```lsp completion_resolve completion [1]
label=count
documentation=Count one value.
```

```lsp completion_resolve completion [2]
label=print
documentation=Print one line.
```

```lsp completion_resolve completion [3]
label=play
documentation=Play one tone.
```
