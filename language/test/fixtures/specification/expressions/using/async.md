# Async

`await using` disposes resources asynchronously at scope exit.

## await using

### await using accepts async resources

`await using` can appear in async scopes and accepts `AsyncDispose`.

```ds
class Connection implements AsyncDispose {
    asyncDispose(): Promise<void> {
        Promise.resolve()
    }
}

async function run(): Promise<void> {
    await using connection = new Connection();
    connection satisfies Connection;
}
```

### await using accepts sync resources

`await using` falls back to `Dispose` for synchronous resources.

```ds
class File implements Dispose {
    dispose(): void {}
}

async function run(): Promise<void> {
    await using file = new File();
    file satisfies File;
}
```

### await using accepts nullish resources

`null` and `undefined` are ignored by async resource cleanup.

```ds
async function run(): Promise<void> {
    await using missing = null;
    await using absent = undefined;
}
```

### await using rejects non disposable values

`await using` still requires a disposable resource.

```ds
async function run(): Promise<void> {
    await using value = 1;
}
```

- contains: Dispose

### await using requires async scopes

`await using` can only appear where `await` is allowed.

```ds
function run(): void {
    await using value = null;
}
```

- contains: await
