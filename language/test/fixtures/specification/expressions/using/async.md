# Async

## await using

### await using accepts async resources

> `await using` can appear in async scopes and accepts `AsyncDispose`.

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

> `await using` falls back to `Dispose` for synchronous resources.

```ds
class File implements Dispose {
    dispose(): void {}
}

async function run(): Promise<void> {
    await using file = new File();
    file satisfies File;
}
```
