# Raw Memory

## pointers

### raw pointers are non-null

`*T` is already non-null.
Use a union when null or absence is a real state.

```ds
declare function pointer(): *int32;
declare function optionalPointer(): *int32 | undefined;

let value = pointer();
let optional = optionalPointer();

value satisfies *int32;
optional satisfies *int32 | undefined;
```

### unsafe operations use raw pointers directly

Raw operations remain outside the ownership model and must be requested explicitly.

```ds
import { read, write } from "destack:memory";

declare function source(): *int32;
declare function destination(): *int32;

@allowUnsafe
function copyOne(): ^int32 {
    let value = read(source());
    write(destination(), value);
    return read(destination());
}

let copied = copyOne();

copied satisfies ^int32;
```

### trusted wrappers hide unsafe implementations

The wrapper keeps the unsafe boundary local and exposes an ordinary safe API.

```ds
import { asReadonlyReference } from "destack:memory";

@trusted
@allowUnsafe
function at(pointer: *int32): &readonly int32 {
    return asReadonlyReference(pointer);
}
```

## layout

### layouts can be built from types

Allocator users can ask for the exact layout of typed storage.

```ds
import { AllocationLayout } from "destack:memory";

struct Point {
    x: float64;
    y: float64;
}

let layout = AllocationLayout.fromType<Point>();

layout satisfies AllocationLayout;
```
