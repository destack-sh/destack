# Capabilities

## placement

### shared does not imply sync

> Shared placement is separate from `Sync`.

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

> APIs that cross Worker boundaries can require `Send`.

```ds
function postToWorker<T: Send>(value: T): void {}

class NotSend {}

postToWorker(new NotSend());
```

- contains: Send
