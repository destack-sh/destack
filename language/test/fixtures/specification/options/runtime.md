# Runtime Restriction Options

Tests for options that restrict dynamic runtime features.

## noDynamicImport

### noDynamicImport reports dynamic imports when true

> Dynamic import expressions are rejected when noDynamicImport is true.

```ds:main.ds
import("./example.ds");
```

```ds:example.ds
export const value = 1;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noDynamicImport": true } }
```

- dynamic imports are disabled

### noDynamicImport allows dynamic imports when false

> Dynamic import expressions are allowed when noDynamicImport is false.

```ds:main.ds
import("./example.ds");
```

```ds:example.ds
export const value = 1;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noDynamicImport": false } }
```

## noInternalImport

### noInternalImport reports internal protocol imports when true

> Internal protocol imports are rejected when noInternalImport is true.

```ds:main.ds runtime=native emit=native libs=es5
import "platform:fs";
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noInternalImport": true } }
```

- contains: internal module import

### noInternalImport allows internal protocol imports when false

> Internal protocol imports are allowed when noInternalImport is false.

```ds:main.ds runtime=native emit=native libs=es5
import "platform:fs";
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noInternalImport": false } }
```

### noInternalImport reports warnings for internal protocol imports when warn

> Internal protocol imports emit warnings when noInternalImport is warn.

```ds:main.ds runtime=native emit=native libs=es5
import "platform:fs";
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noInternalImport": "warn" } }
```

- warning: internal module import

## noDynamicEvaluation

### noDynamicEvaluation reports eval when true

> Eval calls are rejected when noDynamicEvaluation is true.

```ds:main.ds libs=es2020
eval("1");
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noAny": false, "noDynamicEvaluation": true } }
```

- dynamic evaluation is disabled

### noDynamicEvaluation allows eval when false

> Eval calls are allowed when noDynamicEvaluation is false.

```ds:main.ds libs=es2020
eval("1");
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noAny": false, "noDynamicEvaluation": false } }
```

### noDynamicEvaluation reports Function when true

> Function constructors are rejected when noDynamicEvaluation is true.

```ds:main.ds libs=es2020
let value = Function(["return 1"]);
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noAny": false, "noDynamicEvaluation": true } }
```

- dynamic evaluation is disabled

### noDynamicEvaluation allows Function when false

> Function constructors are allowed when noDynamicEvaluation is false.

```ds:main.ds libs=es2020
let value = Function(["return 1"]);
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noAny": false, "noDynamicEvaluation": false } }
```

## noProxy

### noProxy reports Proxy usage when true

> Proxy construction is rejected when noProxy is true.

```ds:main.ds libs=es2020
declare const handler: ProxyHandler<object>;
declare const target: object;
let proxy = new Proxy(target, handler);
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noAny": false, "noProxy": true } }
```

- proxy usage is disabled

### noProxy allows Proxy usage when false

> Proxy construction is allowed when noProxy is false.

```ds:main.ds libs=es2020
declare const handler: ProxyHandler<object>;
declare const target: object;
let proxy = new Proxy(target, handler);
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noAny": false, "noProxy": false } }
```

## noDynamicShapes

### noDynamicShapes reports shape mutation when true

> Dynamic shape mutation is rejected when noDynamicShapes is true.

```ds:main.ds libs=es2020
declare const descriptor: PropertyDescriptor;
let target = { value: 1 };
Object.defineProperty(target, "x", descriptor);
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noAny": false, "noDynamicShapes": true } }
```

- dynamic shape mutation is disabled

### noDynamicShapes allows shape mutation when false

> Dynamic shape mutation is allowed when noDynamicShapes is false.

```ds:main.ds libs=es2020
declare const descriptor: PropertyDescriptor;
let target = { value: 1 };
Object.defineProperty(target, "x", descriptor);
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noAny": false, "noDynamicShapes": false } }
```

### noDynamicShapes reports delete when true

> Deleting properties is rejected when noDynamicShapes is true.

```ds:main.ds
let target = { value: 1 };
delete target.value;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noDynamicShapes": true } }
```

- dynamic shape mutation is disabled

### noDynamicShapes allows delete when false

> Deleting properties is allowed when noDynamicShapes is false.

```ds:main.ds
let target = { value: 1 };
delete target.value;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noDynamicShapes": false } }
```

## noManaged

### noManaged reports managed allocations when true

> Managed allocations are rejected when noManaged is true.

```ds:main.ds
class Box {
    value: number = 0;
}

let value = new Box();
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noManaged": true } }
```

- managed memory is disabled

### noManaged allows explicit ownership annotations

> Explicit ownership operators are allowed when noManaged is true.

```ds:main.ds
class Box {
    value: number = 0;
}

let value2: ^Box = ^new Box();
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noManaged": true } }
```

## noRuntime

### noRuntime reports async functions when true

> Runtime features are rejected when noRuntime is true.

```ds:main.ds
async function run(): int32 {
    return 1;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noRuntime": true } }
```

- runtime features are disabled

### noRuntime reports await when true

> Await expressions are rejected when noRuntime is true.

```ds:main.ds libs=es2020
declare const promise: any;
const value = await promise;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noAny": false, "noRuntime": true } }
```

- runtime features are disabled

### noRuntime reports yield when true

> Generator functions are rejected when noRuntime is true.

```ds:main.ds
function* generator(): int32 {
    yield 1;
    return 0;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noRuntime": true } }
```

- runtime features are disabled

### noRuntime allows async functions when false

> Async functions are allowed when noRuntime is false.

```ds:main.ds
async function run(): int32 {
    return 1;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noRuntime": false } }
```

### noRuntime allows await when false

> Await expressions are allowed when noRuntime is false.

```ds:main.ds libs=es2020
declare const promise: any;
const value = await promise;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noAny": false, "noRuntime": false } }
```

### noRuntime allows yield when false

> Generator functions are allowed when noRuntime is false.

```ds:main.ds
function* generator(): int32 {
    yield 1;
    return 0;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noRuntime": false } }
```

## noExceptions

### noExceptions reports throw when true

> Throw statements are rejected when noExceptions is true.

```ds:main.ds
function boom(): void {
    throw new Error("no");
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noExceptions": true } }
```

- exceptions are disabled

### noExceptions reports try catch when true

> Try and catch are rejected when noExceptions is true.

```ds:main.ds
try {
    1;
} catch err {
    err;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noExceptions": true } }
```

- exceptions are disabled

### noExceptions allows try catch when false

> Try and catch are allowed when noExceptions is false.

```ds:main.ds
try {
    1;
} catch err {
    err;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noExceptions": false } }
```

## noComputedPropertyAccess

### noComputedPropertyAccess reports computed access when true

> Computed property access is rejected when noComputedPropertyAccess is true.

```ds:main.ds
let target = { value: 1 };
let key = "value";
let out = target[key];
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noComputedPropertyAccess": true } }
```

- computed property access is disabled

### noComputedPropertyAccess allows computed access when false

> Computed property access is allowed when noComputedPropertyAccess is false.

```ds:main.ds
let target: { [key: string]: int32 } = { value: 1 };
let key = "value";
let out = target[key];
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noComputedPropertyAccess": false } }
```

## noReferentialEquality

### noReferentialEquality reports object equality when true

> Referential equality comparisons are rejected when noReferentialEquality is true.

```ds:main.ds
let left = { value: 1 };
let right = { value: 2 };
let same = left == right;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noReferentialEquality": true } }
```

- referential equality is disabled

### noReferentialEquality reports strict equality when true

> Strict referential equality comparisons are rejected when noReferentialEquality is true.

```ds:main.ds
let left = { value: 1 };
let right = { value: 2 };
let same = left === right;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noReferentialEquality": true } }
```

- referential equality is disabled

### noReferentialEquality reports strict inequality when true

> Strict referential inequality comparisons are rejected when noReferentialEquality is true.

```ds:main.ds
let left = { value: 1 };
let right = { value: 2 };
let same = left !== right;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noReferentialEquality": true } }
```

- referential equality is disabled

### noReferentialEquality allows object equality when false

> Referential equality comparisons are allowed when noReferentialEquality is false.

```ds:main.ds
struct Measure { value: int }

extension of Measure implements Equal<Measure> {
    equal(other: Measure): boolean { return true }
}

let left = Measure { value: 1 };
let right = Measure { value: 2 };
let same = left == right;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noReferentialEquality": false } }
```

### noReferentialEquality allows class equality when false

> Referential equality comparisons are allowed for classes when noReferentialEquality is false.

```ds:main.ds
class User {
    name: string = ""
}

let left = new User();
let right = new User();
let same = left === right;
same satisfies boolean;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noReferentialEquality": false } }
```


## noImplicitDynamicDispatch

### noImplicitDynamicDispatch reports union dispatch when true

> Implicit dynamic dispatch is rejected when noImplicitDynamicDispatch is true.

```ds:main.ds
struct Cat {
    name: string;

    speak(): string {
        "meow"
    }
}

struct Dog {
    name: string;

    speak(): string {
        "woof"
    }
}

let pet: Cat | Dog = Cat { name: "Milo" };
pet.speak();
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noImplicitDynamicDispatch": true } }
```

- implicit dynamic dispatch is disabled

### noImplicitDynamicDispatch allows union dispatch when false

> Implicit dynamic dispatch is allowed when noImplicitDynamicDispatch is false.

```ds:main.ds
struct Cat {
    name: string;

    speak(): string {
        "meow"
    }
}

struct Dog {
    name: string;

    speak(): string {
        "woof"
    }
}

let pet: Cat | Dog = Cat { name: "Milo" };
pet.speak();
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noImplicitDynamicDispatch": false } }
```

## noGlobalThis

### noGlobalThis reports globalThis access when true

> globalThis access is rejected when noGlobalThis is true.

```ds:main.ds
declare const globalThis: object;

let value = globalThis;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noGlobalThis": true } }
```

- globalThis access is disabled

### noGlobalThis allows globalThis access when false

> globalThis access is allowed when noGlobalThis is false.

```ds:main.ds
declare const globalThis: object;

let value = globalThis;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "noGlobalThis": false } }
```

## useDefineForClassFields

### useDefineForClassFields accepts class field initializers

> Class field initializers are supported under useDefineForClassFields.

```ts:main.ts
class Counter {
    value = 1;
}

const counter = new Counter();
counter.value satisfies number;
```

```ts:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "useDefineForClassFields": true } }
```

### useDefineForClassFields allows class field initializers when false

> Class field initializers are still accepted when useDefineForClassFields is false.

```ts:main.ts
class Counter {
    value = 1;
}

const counter = new Counter();
counter.value satisfies number;
```

```ts:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "useDefineForClassFields": false } }
```
