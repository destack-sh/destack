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

## exactOptionalPropertyTypes

### exactOptionalPropertyTypes rejects undefined assignments

> Exact optional property types disallow assigning undefined to present fields.

```ts:main.ts
type Box = { value?: string };

const ok: Box = {};
const bad: Box = { value: undefined };
```

```ts:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "exactOptionalPropertyTypes": true, "checkTs": true } }
```

- contains: not assignable

### exactOptionalPropertyTypes allows explicit undefined when declared

> Optional properties that include undefined accept explicit undefined values.

```ts:main.ts
type Box = { value?: string | undefined };

const ok: Box = { value: undefined };
ok.value satisfies string | undefined;
```

```ts:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "exactOptionalPropertyTypes": true, "checkTs": true } }
```

### exactOptionalPropertyTypes allows undefined when false

> Non exact optional property types allow undefined assignments.

```ts:main.ts
type Box = { value?: string };

const ok: Box = { value: undefined };
ok.value satisfies string | undefined;
```

```ts:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "exactOptionalPropertyTypes": false, "checkTs": true } }
```

## noUncheckedIndexedAccess

### noUncheckedIndexedAccess adds undefined to index access

> Index signature access includes undefined when noUncheckedIndexedAccess is true.

```ds:main.ds
interface Bag {
    [key: string]: int32
}

const bag: Bag = { a: 1 };
const value: int32 = bag["missing"];
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noUncheckedIndexedAccess": true } }
```

- contains: not assignable

### noUncheckedIndexedAccess allows index access when false

> Index signature access stays exact when noUncheckedIndexedAccess is false.

```ds:main.ds
interface Bag {
    [key: string]: int32
}

const bag: Bag = { a: 1 };
const value: int32 = bag["missing"];
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noUncheckedIndexedAccess": false } }
```

### noUncheckedIndexedAccess does not affect arrays

> Array element access does not add undefined for noUncheckedIndexedAccess.

```ds:main.ds
const values: int32[] = [1, 2, 3];
const value: int32 = values[0];
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noUncheckedIndexedAccess": true } }
```

## noPropertyAccessFromIndexSignature

### noPropertyAccessFromIndexSignature rejects property access

> Property access is rejected when noPropertyAccessFromIndexSignature is true.

```ds:main.ds
interface Bag {
    [key: string]: int32
}

const bag: Bag = { a: 1 };
const value = bag.missing;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noPropertyAccessFromIndexSignature": true } }
```

- contains: index signature

### noPropertyAccessFromIndexSignature allows property access

> Property access is allowed when noPropertyAccessFromIndexSignature is false.

```ds:main.ds
interface Bag {
    [key: string]: int32
}

const bag: Bag = { a: 1 };
const value = bag.missing;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noPropertyAccessFromIndexSignature": false } }
```

### noPropertyAccessFromIndexSignature allows declared properties

> Declared properties are still accessible when noPropertyAccessFromIndexSignature is true.

```ds:main.ds
interface Bag {
    known: int32
    [key: string]: int32
}

const bag: Bag = { known: 1 };
const value: int32 = bag.known;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noPropertyAccessFromIndexSignature": true } }
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

## noMustAssertions

### noMustAssertions reports must assertions when true

> Must assertions are rejected when noMustAssertions is true.

```ds:main.ds
declare const value: string | undefined;

let out = value!;
out;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noMustAssertions": true } }
```

- contains: must assertions are disabled

### noMustAssertions allows must assertions when false

> Must assertions are allowed when noMustAssertions is false.

```ds:main.ds
declare const value: string | undefined;

let out = value!;
out satisfies string;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noMustAssertions": false } }
```

## noDefiniteAssignmentAssertions

### noDefiniteAssignmentAssertions reports definite assignment when true

> Definite assignment assertions are rejected when noDefiniteAssignmentAssertions is true.

```ds:main.ds
class User {
    name!: string = "";
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noDefiniteAssignmentAssertions": true } }
```

- contains: definite assignment assertions are disabled

### noDefiniteAssignmentAssertions allows definite assignment when false

> Definite assignment assertions are allowed when noDefiniteAssignmentAssertions is false.

```ds:main.ds
class User {
    name!: string = "";
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noDefiniteAssignmentAssertions": false } }
```

## noCustomTypeGuards

### noCustomTypeGuards reports custom type guards when true

> Custom type guards are rejected when noCustomTypeGuards is true.

```ds:main.ds
declare function isString(value: unknown): value is string;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noCustomTypeGuards": true } }
```

- contains: custom type guards are disabled

### noCustomTypeGuards allows custom type guards when false

> Custom type guards are allowed when noCustomTypeGuards is false.

```ds:main.ds
declare function isString(value: unknown): value is string;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noCustomTypeGuards": false } }
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

## noUnsoundVariance

### noUnsoundVariance reports mutable array covariance

> Mutable array covariance is rejected when noUnsoundVariance is true.

```ds:main.ds
class Animal {}
class Dog extends Animal {}

let dogs: Dog[] = [];
let animals: Animal[] = dogs;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noUnsoundVariance": true } }
```

- contains: unsound variance is disabled

### noUnsoundVariance allows mutable array covariance when false

> Mutable array covariance is allowed when noUnsoundVariance is false.

```ds:main.ds
class Animal {}
class Dog extends Animal {}

let dogs: Dog[] = [];
let animals: Animal[] = dogs;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noUnsoundVariance": false } }
```

## noUnsoundNarrowing

### noUnsoundNarrowing reports instanceof guards

> Instanceof narrowing is rejected when noUnsoundNarrowing is true.

```ds:main.ds
class Animal {}

let value: unknown = new Animal();
if (value instanceof Animal) {
    value;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noUnsoundNarrowing": true } }
```

- contains: unsound narrowing is disabled

### noUnsoundNarrowing allows guards when false

> Instanceof narrowing is allowed when noUnsoundNarrowing is false.

```ds:main.ds
class Animal {}

let value: unknown = new Animal();
if (value instanceof Animal) {
    value;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "noUnsoundNarrowing": false } }
```

## deepReadonly

### deepReadonly rewrites nested readonly fields when true

> Nested fields become readonly when deepReadonly is true.

```ds:main.ds
let wrapped: readonly int32[][] = [[1]];
wrapped[0][0] = 2;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "deepReadonly": true } }
```

- contains: cannot assign to readonly property

### deepReadonly allows shallow readonly when false

> Nested fields remain mutable when deepReadonly is false.

```ds:main.ds
let wrapped: readonly int32[][] = [[1]];
wrapped[0][0] = 2;
```

```ds:package.json
{ "name": "spec" }
```

```json:dsconfig.json
{ "compilerOptions": { "deepReadonly": false } }
```

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
