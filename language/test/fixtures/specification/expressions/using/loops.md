# Loops

Using loop bindings dispose each resource at the end of its iteration.

## for of

### for of accepts using resources

```ds
class File implements Dispose {
    dispose(): void {}
}

declare const files: File[];

for (using file of files) {
    file satisfies File;
}
```

### for of accepts nullish using resources

```ds
class File implements Dispose {
    dispose(): void {}
}

declare const files: (File | null | undefined)[];

for (using file of files) {
    file satisfies File | null | undefined;
}
```

### for of rejects async-only using resources

```ds
class Connection implements AsyncDispose {
    asyncDispose(): Promise<void> {
        Promise.resolve()
    }
}

declare const connections: Connection[];

for (using connection of connections) {
    connection satisfies Connection;
}
```

- contains: Dispose

### for in rejects using resources

```ds
class File implements Dispose {
    dispose(): void {}
}

declare const files: { [path: string]: File };

for (using path in files) {
    path satisfies string;
}
```

- contains: for-in

## for await of

### for await of accepts await using resources

```ds
class Connection implements AsyncDispose {
    asyncDispose(): Promise<void> {
        Promise.resolve()
    }
}

interface AsyncIterator<T> {
    next(): Promise<IteratorResult<T>>;
}

interface AsyncIterable<T> {
    asyncIterator(): AsyncIterator<T>;
}

declare function connections(): AsyncIterable<Connection>;

async function run(): Promise<void> {
    for await (await using connection of connections()) {
        connection satisfies Connection;
    }
}
```

### for await of accepts sync resources

```ds
class File implements Dispose {
    dispose(): void {}
}

declare const files: File[];

async function run(): Promise<void> {
    for await (await using file of files) {
        file satisfies File;
    }
}
```

### await using loop bindings require async iteration

```ds
class File implements Dispose {
    dispose(): void {}
}

declare const files: File[];

for (await using file of files) {
    file satisfies File;
}
```

- contains: await
