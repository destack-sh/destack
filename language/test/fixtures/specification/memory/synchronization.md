# Synchronization

Shared placement is not synchronization; `Sync` APIs and locks are.

## guards

### mutex guards expose exclusive access

The lock is the runtime exclusivity oracle, and its guard dereferences accordingly.

```ds
import { Mutex } from "destack:async";

struct Counter {
    value: int32;
}

let counter: shared Mutex<Counter> = new Mutex(Counter { value: 0 });
const guard = counter.lock();

guard.value = 1;
```

## statics

### shared bindings require sync

A `shared` binding is reachable from every Worker, so its value must be `Sync`.

```ds
class World {}

shared const world: World = new World();
```

- contains: Sync

### locks make shared globals mutable

Global mutable state goes through a lock in the binding.

```ds
import { Mutex } from "destack:async";

struct Counter {
    value: int32;
}

shared const counter: Mutex<Counter> = new Mutex(Counter { value: 0 });

const guard = counter.lock();
guard.value = 1;
```

### owned globals are rejected

Every Worker can reach the binding, so its exclusivity cannot be checked.

```ds
struct Counter {
    value: int32;
}

shared const counter: ^Counter = ^Counter { value: 0 };
```

- contains: Sync
