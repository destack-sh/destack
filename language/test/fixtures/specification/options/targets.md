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

```json:destack.json
{
  "compiler": {
    "noAny": false
  },
  "targets": {
    "native": { "emit": "native" }
  },
  "defaultTarget": "native"
}
```

- any type is disabled

### native targets enforce noImprecisePrimitives

> Native targets forbid imprecise primitive types.

```ds:main.ds
let value: number = 1;
value;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{
  "compiler": {
    "noImprecisePrimitives": false
  },
  "targets": {
    "native": { "emit": "native" }
  },
  "defaultTarget": "native"
}
```

- imprecise primitive type is disabled

### native targets enforce noImplicitConversions

> Native targets forbid implicit numeric conversions.

```ds:main.ds
let value: float64 = 1;
value;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{
  "compiler": {
    "noImplicitConversions": false
  },
  "targets": {
    "native": { "emit": "native" }
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

```json:destack.json
{
  "compiler": {
    "noUnsafeTypeAssertions": false
  },
  "targets": {
    "native": { "emit": "native" }
  },
  "defaultTarget": "native"
}
```

- unsafe type assertions are disabled
- cannot cast type int32 to boolean

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

```json:destack.json
{
  "compiler": {
    "noManaged": false
  },
  "targets": {
    "native": { "emit": "native" }
  },
  "defaultTarget": "native"
}
```

- managed memory is disabled

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

```json:destack.json
{
  "compiler": {
    "noAny": false
  },
  "targets": {
    "wasm": { "emit": "wasm" }
  },
  "defaultTarget": "wasm"
}
```

- any type is disabled