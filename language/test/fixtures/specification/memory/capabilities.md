# Capabilities

Capabilities gate what APIs may do with a value: duplicate it, share it, or move it across Workers.

## duplication

### copy requires capability

APIs that duplicate implicitly can require `Copy`.

```ds
function duplicate<T: Copy>(value: T): (T, T) {
    return (value, value);
}

struct NotCopy {
    value: int32;
}

duplicate(NotCopy { value: 1 });
```

- contains: Copy

### clone requires capability

APIs that duplicate explicitly can require `Clone`.

```ds
function cloneValue<T: Clone>(value: T): T {
    return value.clone();
}

struct NotClone {
    value: int32;
}

cloneValue(NotClone { value: 1 });
```

- contains: Clone

## placement

### shared does not imply sync

Shared placement is separate from `Sync`.

```ds
struct Cell {
    value: int32;
}

function requiresSync<T: Sync>(value: T): void {}

let cell: shared Cell = Cell { value: 1 };
requiresSync(cell);
```

- contains: Sync

## transfer

### local managed handles are not send

Local managed handles stay on their Worker.

```ds
function postToWorker<T: Send>(value: T): void {}

class NotSend {}

postToWorker(new NotSend());
```

- contains: Send

### owned values of send data can be sent

Ownership carries across Workers when the payload is `Send`.

```ds
function postToWorker<T: Send>(value: T): void {}

struct Message {
    value: int32;
}

postToWorker(^Message { value: 1 });
```

### mutable borrows never cross workers

Aliased mutability is only sound within one Worker.

```ds
function postToWorker<T: Send>(value: T): void {}

let value: int32 = 1;
let borrow = &value;

postToWorker(borrow);
```

- contains: Send

### readonly borrows of owned sync data can cross

A readonly borrow crosses when the payload is `Sync` and the source is owned.

```ds
function publish<T: Sync>(value: T): void {}

struct Stats {
    total: int32;
}

let stats = ^Stats { total: 1 };

publish(&readonly stats);
```

## derivation

### send derives structurally

A struct of `Send` fields is `Send`.

```ds
function postToWorker<T: Send>(value: T): void {}

struct Pair {
    left: int32;
    right: bool;
}

postToWorker(^Pair { left: 1, right: true });
```

### send derivation stops at managed fields

A struct holding a local managed handle is not `Send`.

```ds
function postToWorker<T: Send>(value: T): void {}

class Connection {}

struct Holder {
    connection: Connection;
}

declare const holder: ^Holder;

postToWorker(holder);
```

- contains: Send
