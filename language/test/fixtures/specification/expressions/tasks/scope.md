# Task Scopes

A `TaskScope` owns spawned tasks and cannot exit while children are pending.

## structure

### scopes own spawned tasks

`spawn` returns a `Task` owned by the scope.

```ds
declare function fetchName(): Promise<string>;

async function run(): Promise<void> {
    await using scope = TaskScope.open();
    const task = scope.spawn(() => fetchName());

    task satisfies Task<string>;
}
```

### join surfaces cancellation as a result

`join` resolves to the value or `Cancelled`, not an exception.

```ds
declare function fetchName(): Promise<string>;

async function run(): Promise<void> {
    await using scope = TaskScope.open();
    const task = scope.spawn(() => fetchName());

    const result = await task.join();
    result satisfies Result<string, Cancelled>;
}
```

### scopes adopt pending promises

`adopt` re-parents an existing promise into the scope.

```ds
declare function fetchName(): Promise<string>;

async function run(): Promise<void> {
    await using scope = TaskScope.open();
    const task = scope.adopt(fetchName());

    task satisfies Task<string>;
}
```

## cancellation

### cancelled tasks observe cancelled on join

Cancellation ends the task; only `join` sees `Cancelled`.

```ds
declare function fetchName(): Promise<string>;

async function run(): Promise<void> {
    await using scope = TaskScope.open();
    const task = scope.spawn(() => fetchName());

    task.cancel();

    const result = await task.join();
    result satisfies Result<string, Cancelled>;
}
```
