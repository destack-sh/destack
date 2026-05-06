# Sync

## using

### using binding introduces a resource name

`using` introduces a scoped binding whose value implements `Dispose`.

```ds
class File implements Dispose {
    dispose(): void {}
}

using file = new File();
file satisfies File;
```

### using expression yields void

`using` expressions evaluate to `void`.

```ds
class File implements Dispose {
    dispose(): void {}
}

let result: void = using file = new File();
result satisfies void;
```

### using accepts nullish resources

`null` and `undefined` are ignored by resource cleanup.

```ds
using missing = null;
using absent = undefined;
```

### using rejects non disposable values

Ordinary values are not resources.

```ds
using value = 1;
```

- contains: Dispose

### using rejects async-only resources

`using` requires synchronous disposal.

```ds
class Connection implements AsyncDispose {
    asyncDispose(): Promise<void> {
        Promise.resolve()
    }
}

using connection = new Connection();
```

- contains: Dispose

### using rejects declare

Declare bindings cannot have initializers.

```ds
declare using value = null;
```

- contains: declare bindings cannot have initializers
