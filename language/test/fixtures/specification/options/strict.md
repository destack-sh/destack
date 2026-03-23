# Strict Mode

Tests for strict-mode defaults that map to TypeScript strictness.

## strict

### strict enables strict defaults

> Strict mode defaults enable noImplicitAny without explicit flags.

```ds:main.ds
function handle(value) {
    return value;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": true } }
```

- implicit any type

## noImplicitAny

### noImplicitAny rejects implicit any when true

> Explicit noImplicitAny rejects implicit any usages.

```ds:main.ds
function handle(value) {
    return value;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": false, "noImplicitAny": true } }
```

- implicit any type

### noImplicitAny allows implicit any when false

> Explicit noImplicitAny false permits implicit any usages.

```ds:main.ds
function handle(value) {
    return value;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": true, "noImplicitAny": false } }
```

### strict false allows implicit any by default

> Non-strict mode does not enable noImplicitAny by default.

```ds:main.ds
function handle(value) {
    return value;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": false } }
```

### strict enables strict null checks by default

> Strict mode defaults reject null assignments without explicit strictNullChecks.

```ds:main.ds
let value: int32 = null;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": true } }
```

- contains: not assignable

### strict false allows null assignments by default

> Non-strict mode defaults allow null assignments without explicit strictNullChecks.

```ds:main.ds
let value: int32 = null;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": false } }
```

## strictNullChecks

### strictNullChecks rejects null assignments when true

> Explicit strictNullChecks rejects null assignments.

```ds:main.ds
let value: int32 = null;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": false, "strictNullChecks": true } }
```

- contains: not assignable

### strictNullChecks allows null assignments when false

> Explicit strictNullChecks false permits null assignments.

```ds:main.ds
let value: int32 = null;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": true, "strictNullChecks": false } }
```

### strict enables noImplicitThis by default

> Strict mode defaults reject implicit `this` without explicit noImplicitThis.

```ds:main.ds
function counter() {
    this;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": true } }
```

- implicit this type

### strict false allows implicit this by default

> Non-strict mode defaults allow implicit `this` without explicit noImplicitThis.

```ds:main.ds
function counter() {
    this;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": false } }
```

## noImplicitThis

### noImplicitThis reports implicit this when true

> Explicit noImplicitThis rejects implicit this usage.

```ds:main.ds
function counter() {
    this;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": false, "noImplicitThis": true } }
```

- implicit this type

### noImplicitThis allows implicit this when false

> Explicit noImplicitThis false permits implicit this usage.

```ds:main.ds
function counter() {
    this;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": false, "noImplicitThis": false } }
```

### strict enables strictFunctionTypes by default

> Strict mode defaults reject narrow parameter types in function assignability.

```ds:main.ds
interface FnWide {
    (value: string | number): void
}

function narrow(value: string): void {}
let wide: FnWide = narrow;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": true } }
```

- contains: not assignable

## strictFunctionTypes

### strictFunctionTypes rejects narrow parameter types when true

> Explicit strictFunctionTypes rejects narrow parameter assignments.

```ds:main.ds
interface FnWide {
    (value: string | number): void
}

function narrow(value: string): void {}
let wide: FnWide = narrow;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": false, "strictFunctionTypes": true } }
```

- contains: not assignable

### strictFunctionTypes rejects narrow method parameters when true

> Method parameters are checked strictly when strictFunctionTypes is enabled.

```ds:main.ds
type Wide = {
    handle(value: string | number): void
};

class Narrow {
    handle(value: string): void {}
}

let value: Wide = new Narrow();
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": false, "strictFunctionTypes": true } }
```

- contains: not assignable

### strictFunctionTypes allows narrow parameter types when false

> Explicit strictFunctionTypes false permits narrow parameter assignments.

```ds:main.ds
interface FnWide {
    (value: string | number): void
}

function narrow(value: string): void {}
let wide: FnWide = narrow;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": false, "strictFunctionTypes": false } }
```

### strictFunctionTypes allows narrow method parameters when false

> Method parameters are bivariant when strictFunctionTypes is disabled.

```ds:main.ds
type Wide = {
    handle(value: string | number): void
};

class Narrow {
    handle(value: string): void {}
}

let value: Wide = new Narrow();
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": false, "strictFunctionTypes": false } }
```

### strict enables strictBindCallApply by default

> Strict mode defaults enforce `call` argument checks without explicit flags.

```ds:main.ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

add.call({ base: "no" }, 1);
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": true } }
```

- contains: not assignable

## strictBindCallApply

### strictBindCallApply rejects invalid call arguments when true

> Explicit strictBindCallApply enforces call argument checks.

```ds:main.ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

add.call({ base: "no" }, 1);
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": false, "strictBindCallApply": true } }
```

- contains: not assignable

### strictBindCallApply allows invalid call arguments when false

> Explicit strictBindCallApply false permits invalid call arguments.

```ds:main.ds
function add(this: { base: number }, value: number): number {
    return this.base + value;
}

add.call({ base: "no" }, 1);
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": false, "strictBindCallApply": false } }
```

### strict enables strictBuiltinIteratorReturn by default

> Strict mode defaults `BuiltinIteratorReturn` to `undefined`.

```ds:main.ds libs=es5,es2015.iterable
type ReturnValue = BuiltinIteratorReturn;

const value: ReturnValue = 1;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{
  "compiler": { "strict": true, "lib": ["es5", "es2015.iterable"] }
}
```

- contains: not assignable

### strict enables strictPropertyInitialization by default

> Strict mode defaults require class fields to be initialized.

```ds:main.ds
class Counter {
    value: number;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": true } }
```

- property is not definitely assigned

### strict enables useUnknownInCatchVariables by default

> Strict mode defaults catch variables to unknown.

```ds:main.ds
const value = try {
    1
} catch e {
    e satisfies string;
    0
};
value satisfies int;
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": true } }
```

- contains: expected string

## useUnknownInCatchVariables

### useUnknownInCatchVariables reports when true

> Catch variables are unknown when useUnknownInCatchVariables is true.

```ds:main.ds
try {
    throw 1;
} catch (err) {
    err satisfies string;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{ "compiler": { "strict": false, "useUnknownInCatchVariables": true } }
```

- contains: expected string

### useUnknownInCatchVariables allows any when false

> Catch variables are any when useUnknownInCatchVariables is false.

```ds:main.ds
try {
    throw 1;
} catch (err) {
    err satisfies string;
}
```

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{
  "compiler": {
    "strict": false,
    "noAny": false,
    "useUnknownInCatchVariables": false
  }
}
```

## strictBuiltinIteratorReturn

### strict builtin iterator return is undefined

> Builtin iterator return defaults to `undefined` in strict mode.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{
  "compiler": {
    "strictBuiltinIteratorReturn": true,
    "lib": ["es5", "es2015.iterable"]
  }
}
```

```ds libs=es5,es2015.iterable
type ReturnValue = BuiltinIteratorReturn;

const value: ReturnValue = 1;
```

- contains: not assignable

### non-strict builtin iterator return is any

> Builtin iterator return defaults to `any` when strict mode is off.

```ds:package.json
{ "name": "spec" }
```

```json:destack.json
{
  "compiler": {
    "noAny": false,
    "strictBuiltinIteratorReturn": false,
    "lib": ["es5", "es2015.iterable"]
  }
}
```

```ds libs=es5,es2015.iterable
type ReturnValue = BuiltinIteratorReturn;

const value: ReturnValue = 1;
```
