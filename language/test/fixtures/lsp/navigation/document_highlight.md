# Document Highlight

## Local Bindings

### Highlight one local value

Document highlights should mark the local occurrences of the selected value.

```ds:main.ds
const [|/*highlight*/value|] = 1;
const next = [|value|] + [|value|];
```

### Highlight imported bindings inside one file

Document highlights should include the local import binding and every same-file use.

```ds:lib.ds
export function ping(): void {}
```

```ds:main.ds
import { [|/*highlight*/ping|] } from "./lib.ds";

const first = [|ping|];
[|ping|]();
```

### Highlight only the shadowed local binding

Document highlights should respect shadowing and avoid the outer binding.

```ds:main.ds
const outer = 1;

function test(): int32 {
    const [|/*highlight*/outer|] = 2;
    return [|outer|];
}
```

### Highlight field definitions and accesses

Document highlights should include a field declaration and all accesses to the same field.

```ds:main.ds
struct Point {
    [|/*highlight*/x|]: int32
    y: int32
}

function main(point: Point): int32 {
    const first = point.[|x|];
    return point.[|x|] + point.[|x|];
}
```
