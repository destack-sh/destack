# Target Defaults

Targets may enforce strict and soundness defaults regardless of configuration.

## native targets

### native targets enforce noAny

> Native targets forbid `any` even when compiler options disable it.

```ds:main.ds
let value: any = 1;
value;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{
  "compilerOptions": {
    "noAny": false
  },
  "targets": {
    "native": { "output": "native" }
  },
  "defaultTarget": "native"
}
```

- contains: any type is disabled

### native targets enforce noImprecisePrimitives

> Native targets forbid imprecise primitive types.

```ds:main.ds
let value: number = 1;
value;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{
  "compilerOptions": {
    "noImprecisePrimitives": false
  },
  "targets": {
    "native": { "output": "native" }
  },
  "defaultTarget": "native"
}
```

- contains: imprecise primitive type is disabled

### native targets enforce noImplicitConversions

> Native targets forbid implicit numeric conversions.

```ds:main.ds
let value: float64 = 1;
value;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{
  "compilerOptions": {
    "noImplicitConversions": false
  },
  "targets": {
    "native": { "output": "native" }
  },
  "defaultTarget": "native"
}
```

- contains: type 1 is not assignable

### native targets enforce noUnsafeTypeAssertions

> Native targets reject unsafe assertions even when disabled.

```ds:main.ds
let value: int32 = 1;
let cast = value as boolean;
cast;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{
  "compilerOptions": {
    "noUnsafeTypeAssertions": false
  },
  "targets": {
    "native": { "output": "native" }
  },
  "defaultTarget": "native"
}
```

- contains: unsafe type assertions are disabled
- contains: cannot cast type int32 to boolean

### native targets enforce noManaged

> Native targets forbid managed allocations even when compiler options disable it.

```ds:main.ds
class Box {
    value: int32 = 0;
}

let value = new Box();
value;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{
  "compilerOptions": {
    "noManaged": false
  },
  "targets": {
    "native": { "output": "native" }
  },
  "defaultTarget": "native"
}
```

- contains: managed memory is disabled

## wasm targets

### wasm targets enforce noAny

> WASM targets forbid `any` even when compiler options disable it.

```ds:main.ds
let value: any = 1;
value;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{
  "compilerOptions": {
    "noAny": false
  },
  "targets": {
    "wasm": { "output": "wasm" }
  },
  "defaultTarget": "wasm"
}
```

- contains: any type is disabled
