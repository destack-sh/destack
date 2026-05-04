# Managed

## directives

### noManaged rejects managed allocations

> `@noManaged` forbids managed allocations inside the annotated body.

```ds:main.ds
class Box {
    value: number = 0;
}

@noManaged
function run(): void {
    let value = new Box();
}
```

- contains: managed memory is disabled

### noManaged allows owned values

> `@noManaged` allows owned value construction.

```ds:main.ds
struct Data {
    value: int32;
}

@noManaged
function run(): void {
    let value = ^Data { value: 1 };
    value.value;
}
```

### stackOnly rejects managed allocations

> `@stackOnly` forbids managed allocations inside the annotated body.

```ds:main.ds
class Box {
    value: number = 0;
}

@stackOnly
function run(): void {
    let value = new Box();
}
```

- contains: managed memory is disabled

### noManaged rejects managed parameter types

> `@noManaged` forbids managed types in signatures.

```ds:main.ds
class Box {
    value: number = 0;
}

@noManaged
function take(value: Box): void {
}
```

- contains: managed memory is disabled

### noManaged rejects managed return types

> `@noManaged` forbids managed types in signatures.

```ds:main.ds
class Box {
    value: number = 0;
}

@noManaged
function make(): Box {
    return new Box();
}
```

- contains: managed memory is disabled
