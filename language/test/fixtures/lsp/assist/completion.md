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
