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

```json:dsconfig.json
{ "compilerOptions": { "noAny": true } }
```

- contains: any type is disabled

### noAny allows any usage when false

> Explicit any types are allowed when noAny is false.

```ds:main.ds
let value: any = 1;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noAny": false } }
```

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

```json:dsconfig.json
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

```json:dsconfig.json
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

```json:dsconfig.json
{ "compilerOptions": { "noUnknown": true, "useUnknownInCatchVariables": true } }
```

- contains: unknown type is disabled

### noUnknown allows unknown usage when false

> Explicit unknown types are allowed when noUnknown is false.

```ds:main.ds
let value: unknown = 1;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noUnknown": false } }
```

## noImprecisePrimitives

### noImprecisePrimitives reports number usage when true

> Imprecise numeric primitives are rejected when noImprecisePrimitives is true.

```ds:main.ds
let value: number = 1;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
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

```json:dsconfig.json
{ "compilerOptions": { "noAny": false, "noImprecisePrimitives": true } }
```

- contains: imprecise primitive type is disabled

### noImprecisePrimitives allows number usage when false

> Imprecise numeric primitives are allowed when noImprecisePrimitives is false.

```ds:main.ds
let value: number = 1;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noAny": false, "noImprecisePrimitives": false } }
```

## noImplicitConversions

### noImplicitConversions reports implicit numeric conversions when true

> Implicit numeric conversions are rejected when noImplicitConversions is true.

```ds:main.ds
let value: float64 = 1;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noImplicitConversions": true } }
```

- contains: not assignable

### noImplicitConversions allows implicit conversions when false

> Implicit numeric conversions are allowed when noImplicitConversions is false.

```ds:main.ds
let value: float64 = 1;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noImplicitConversions": false } }
```

## implicitCollectionConversions

### implicitCollectionConversions allows record-like conversions when allow

> Record-like conversions are allowed when implicitCollectionConversions is allow.

```ds:main.ds
type Bag = { [key: string]: int32 };
let record: Bag = { alpha: 1 };
let value: int32 | undefined = record["alpha"];
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "implicitCollectionConversions": "allow" } }
```

### implicitCollectionConversions allows sized array conversions when allow

> Sized array conversions are allowed when implicitCollectionConversions is allow.

```ds:main.ds
function take(values: int32[]): int32[] { return values; }
let fixed: int32[2] = [1, 2];
let dynamic = take(fixed);
let value: int32 | undefined = dynamic[0];
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "implicitCollectionConversions": "allow" } }
```

### implicitCollectionConversions warns on record-like conversions when warn

> Record-like conversions emit warnings when implicitCollectionConversions is warn.

```ds:main.ds
type Bag = { [key: string]: int32 };
let record: Bag = { alpha: 1 };
let value: int32 | undefined = record["alpha"];
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "implicitCollectionConversions": "warn" } }
```

- warning: implicit collection conversion

### implicitCollectionConversions warns on sized array conversions when warn

> Sized array conversions emit warnings when implicitCollectionConversions is warn.

```ds:main.ds
function take(values: int32[]): int32[] { return values; }
let fixed: int32[2] = [1, 2];
let dynamic = take(fixed);
let value: int32 | undefined = dynamic[0];
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "implicitCollectionConversions": "warn" } }
```

- warning: implicit collection conversion

### implicitCollectionConversions forbids record-like conversions when deny

> Record-like conversions are rejected when implicitCollectionConversions is deny.

```ds:main.ds
type Bag = { [key: string]: int32 };
let record: Bag = { alpha: 1 };
let value: int32 | undefined = record["alpha"];
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "implicitCollectionConversions": "deny" } }
```

- contains: implicit collection conversions are disabled

### implicitCollectionConversions forbids sized array conversions when deny

> Sized array conversions are rejected when implicitCollectionConversions is deny.

```ds:main.ds
function take(values: int32[]): int32[] { return values; }
let fixed: int32[2] = [1, 2];
let dynamic = take(fixed);
let value: int32 | undefined = dynamic[0];
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "implicitCollectionConversions": "deny" } }
```

- contains: implicit collection conversions are disabled

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

```json:dsconfig.json
{ "compilerOptions": { "noAny": false, "noUnsafeTypeAssertions": true } }
```

- contains: unsafe type assertions are disabled

### noUnsafeTypeAssertions allows safe assertions when true

> Assignable assertions remain allowed when noUnsafeTypeAssertions is true.

```ds:main.ds
let value: int32 = 1;
let cast = value as int32;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noUnsafeTypeAssertions": true } }
```

### noUnsafeTypeAssertions allows unsafe assertions when false

> Unsafe type assertions are allowed when noUnsafeTypeAssertions is false.

```ds:main.ds
let value: any = 1;
let cast = value as int32;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noAny": false, "noUnsafeTypeAssertions": false } }
```

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

```json:dsconfig.json
{ "compilerOptions": { "noImplicitManaged": true } }
```

- contains: implicit managed types are disabled

### noImplicitManaged reports implicit managed array types

> Array types require explicit ownership when noImplicitManaged is true.

```ds:main.ds
let values: int32[] = [1, 2, 3];
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noImplicitManaged": true } }
```

- contains: implicit managed types are disabled

### noImplicitManaged reports implicit managed object types

> Structural object types require explicit ownership when noImplicitManaged is true.

```ds:main.ds
let value: { x: int32 } = { x: 1 };
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noImplicitManaged": true } }
```

- contains: implicit managed types are disabled

### noImplicitManaged reports implicit managed function types

> Function types require explicit ownership when noImplicitManaged is true.

```ds:main.ds
let fn: () => int32 = () => 1;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noImplicitManaged": true } }
```

- contains: implicit managed types are disabled

### noImplicitManaged reports implicit managed string types

> String types require explicit ownership when noImplicitManaged is true.

```ds:main.ds
let value: string = "hello";
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
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

```json:dsconfig.json
{ "compilerOptions": { "noImplicitManaged": true } }
```

- contains: implicit managed values are disabled

### noImplicitManaged reports inferred managed arrays

> Inferred array values require explicit ownership when noImplicitManaged is true.

```ds:main.ds
let values = [1, 2, 3];
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
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

```json:dsconfig.json
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

```json:dsconfig.json
{ "compilerOptions": { "noImplicitManaged": true } }
```
