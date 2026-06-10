# Floating Promises

Everything a frame starts must be awaited, returned, or handed to a scope; discarding a promise is denied by default.

## deny

### discarded promises are denied

A bare promise statement floats.

```ds
declare function refresh(): Promise<void>;

async function run(): Promise<void> {
    refresh();
}
```

- contains: floating

## handled

### awaited promises are handled

Awaiting settles the promise in this frame.

```ds
declare function refresh(): Promise<void>;

async function run(): Promise<void> {
    await refresh();
}
```

### returned promises are handled

Returning hands the promise to the caller's frame.

```ds
declare function refresh(): Promise<void>;

function run(): Promise<void> {
    refresh()
}
```

### spawned promises are handled

Spawning hands the work to a scope.

```ds
declare function refresh(): Promise<void>;

async function run(): Promise<void> {
    await using scope = TaskScope.open();
    scope.spawn(() => refresh());
}
```

### stored promises are handled

Assignment counts as handling, like the TypeScript lint.

```ds
declare function refresh(): Promise<void>;

async function run(): Promise<void> {
    const pending = refresh();
    await pending;
}
```
