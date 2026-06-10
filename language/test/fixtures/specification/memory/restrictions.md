# Restrictions

`@noManaged` and `@noHeap` opt code out of managed allocation.

## managed

### noManaged rejects managed allocations

`@noManaged` forbids managed allocation inside the annotated body.

```ds:main.ds
class Widget {
    value: number = 0;
}

@noManaged
function run(): void {
    let value = new Widget();
}
```

- contains: managed memory is disabled

### noManaged allows owned values

`@noManaged` allows owned value construction.

```ds:main.ds
struct Data {
    value: int32;
}

@noManaged
function run(): void {
    let value = ^Data { value: 1 };
    value.value;
}
```

### noHeap rejects managed allocations

`@noHeap` forbids managed allocation inside the annotated body.

```ds:main.ds
class Widget {
    value: number = 0;
}

@noHeap
function run(): void {
    let value = new Widget();
}
```

- contains: managed memory is disabled

### noHeap rejects fallible heap allocation

`@noHeap` forbids heap allocation through typed fallible APIs.

```ds:main.ds
import { Array } from "destack:collections";

@noHeap
function run(): void {
    let value = Array<int32>.tryWithCapacity(64)?;
}
```

- contains: noHeap forbids heap allocations

### noManaged rejects managed parameters

`@noManaged` forbids managed types in signatures.

```ds:main.ds
class Widget {
    value: number = 0;
}

@noManaged
function take(value: Widget): void {}
```

- contains: managed memory is disabled

### noManaged rejects managed returns

`@noManaged` forbids managed return types.

```ds:main.ds
class Widget {
    value: number = 0;
}

@noManaged
function make(): Widget {
    return new Widget();
}
```

- contains: managed memory is disabled
