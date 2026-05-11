# Capabilities

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

### worker transfer can require send

APIs that cross Worker boundaries can require `Send`.

```ds
function postToWorker<T: Send>(value: T): void {}

class NotSend {}

postToWorker(new NotSend());
```

- contains: Send
