## Managed Directives

### @noManaged rejects managed allocations

> noManaged decorators forbid managed allocations inside the annotated body.

```ds:main.ds
class Box {
    value: number = 0;
}

@noManaged
function run(): void {
    let value = new Box();
}
```

```ds:package.json
{ "name": "spec" }
```

- contains: managed memory is disabled

### @stackOnly rejects managed allocations

> stackOnly decorators forbid managed allocations inside the annotated body.

```ds:main.ds
class Box {
    value: number = 0;
}

@stackOnly
function run(): void {
    let value = new Box();
}
```

```ds:package.json
{ "name": "spec" }
```

- contains: managed memory is disabled

### @noManaged rejects managed parameter types

> noManaged decorators forbid managed types in signatures.

```ds:main.ds
class Box {
    value: number = 0;
}

@noManaged
function take(value: Box): void {
}
```

```ds:package.json
{ "name": "spec" }
```

- contains: managed memory is disabled

### @noManaged rejects managed return types

> noManaged decorators forbid managed types in signatures.

```ds:main.ds
class Box {
    value: number = 0;
}

@noManaged
function make(): Box {
    return new Box();
}
```

```ds:package.json
{ "name": "spec" }
```

- contains: managed memory is disabled
