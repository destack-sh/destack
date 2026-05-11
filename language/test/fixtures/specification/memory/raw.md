# Raw Memory

## pointers

### non-null pointers can be checked

`NonNull` makes the null check explicit before raw pointer operations continue.

```ds
import { NonNull } from "destack:memory";

declare function pointer(): *int32;

let value = NonNull.fromRaw(pointer());

value satisfies NonNull<int32> | undefined;
```

### non-null pointers expose primitive operations

Raw operations remain outside the ownership model and must be requested explicitly.

```ds
import { NonNull } from "destack:memory";

declare function destination(): NonNull<int32>;

let pointer = destination();
pointer.write(^1);
let value = pointer.read();

value satisfies ^int32;
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
