# Raw Memory

Raw pointers are inert addresses; only using them is unsafe.

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

@unsafe
function copyOne(): int32 {
    let value = read(source());
    write(destination(), value);
    return read(destination());
}

@unsafe
function run(): void {
    let copied = copyOne();

    copied satisfies int32;
}
```

### safe wrappers hide unsafe implementations

The wrapper keeps the unsafe boundary local and exposes an ordinary safe API.

```ds
import { asReadonlyReference } from "destack:memory";

@safe
function at(pointer: *int32): &readonly int32 {
    return asReadonlyReference(pointer);
}
```

## allocation

### raw pointers do not allocate

Raw pointers are non-owning views over storage created elsewhere.

```ds
struct Point {
    x: float64;
    y: float64;
}

declare function source(): &Point;

let pointer: *Point = source();

pointer satisfies *Point;
```
