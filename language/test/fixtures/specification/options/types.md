# Type Safety Options

Tests for options that restrict type usage and assertions.

## noAny

### noAny reports any usage when true

> Explicit any types are rejected when noAny is true.

```ds:main.ds
let value: any = 1;
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noAny": true } }
```

- contains: any type is disabled

### noAny reports inferred any in catch variables

> Inferred any types are rejected when noAny is true.

```ds:main.ds
try {
    throw 1;
} catch (err) {
    err;
}
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noAny": true, "useUnknownInCatchVariables": false } }
```

- contains: any type is disabled

## noUnknown

### noUnknown reports unknown usage when true

> Explicit unknown types are rejected when noUnknown is true.

```ds:main.ds
let value: unknown = 1;
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noUnknown": true } }
```

- contains: unknown type is disabled

### noUnknown reports inferred unknown types

> Inferred unknown types are rejected when noUnknown is true.

```ds:main.ds
try {
    throw 1;
} catch (err) {
    err;
}
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noUnknown": true, "useUnknownInCatchVariables": true } }
```

- contains: unknown type is disabled

## noImprecisePrimitives

### noImprecisePrimitives reports number usage when true

> Imprecise numeric primitives are rejected when noImprecisePrimitives is true.

```ds:main.ds
let value: number = 1;
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noAny": false, "noImprecisePrimitives": true } }
```

- contains: imprecise primitive type is disabled

### noImprecisePrimitives reports inferred number usage

> Inferred number types are rejected when noImprecisePrimitives is true.

```ds:main.ds libs=es5
let value = Number(1);
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noAny": false, "noImprecisePrimitives": true } }
```

- contains: imprecise primitive type is disabled

## noImplicitConversions

### noImplicitConversions reports implicit numeric conversions when true

> Implicit numeric conversions are rejected when noImplicitConversions is true.

```ds:main.ds
let value: float64 = 1;
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitConversions": true } }
```

- contains: not assignable

## noUnsafeTypeAssertions

### noUnsafeTypeAssertions reports unsafe assertions when true

> Unsafe type assertions are rejected when noUnsafeTypeAssertions is true.

```ds:main.ds
let value: any = 1;
let cast = value as int32;
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noAny": false, "noUnsafeTypeAssertions": true } }
```

- contains: unsafe type assertions are disabled

## noImplicitManaged

### noImplicitManaged reports implicit managed types in annotations

> Managed defaults are rejected in type positions when noImplicitManaged is true.

```ds:main.ds
class Box {
    value: number = 0;
}

let value: Box = new Box();
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitManaged": true } }
```

- contains: implicit managed types are disabled

### noImplicitManaged reports inferred managed values

> Inferred managed values require explicit ownership when noImplicitManaged is true.

```ds:main.ds
class Box {
    value: number = 0;
}

let value = new Box();
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitManaged": true } }
```

- contains: implicit managed values are disabled

### noImplicitManaged allows explicit ownership

> Explicit ownership annotations satisfy noImplicitManaged.

```ds:main.ds
class Box {
    value: number = 0;
}

let value: ^Box = ^(new Box());
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitManaged": true } }
```

### noImplicitManaged allows character literals

> Character values are value types and do not require explicit ownership.

```ds:main.ds
let value: character = 'a';
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitManaged": true } }
```
