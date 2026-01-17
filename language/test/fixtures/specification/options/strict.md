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

```ds:dsconfig.json
{ "compilerOptions": { "strict": true } }
```

- contains: implicit any type

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

```ds:dsconfig.json
{ "compilerOptions": { "strict": false } }
```

### strict enables strict null checks by default

> Strict mode defaults reject null assignments without explicit strictNullChecks.

```ds:main.ds
let value: int32 = null;
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "strict": true } }
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

```ds:dsconfig.json
{ "compilerOptions": { "strict": false } }
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

```ds:dsconfig.json
{ "compilerOptions": { "strict": true } }
```

- contains: implicit this type

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

```ds:dsconfig.json
{ "compilerOptions": { "strict": false } }
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

```ds:dsconfig.json
{ "compilerOptions": { "strict": true } }
```

- contains: not assignable

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

```ds:dsconfig.json
{ "compilerOptions": { "strict": true } }
```

- contains: not assignable

### strict enables strictBuiltinIteratorReturn by default

> Strict mode defaults `BuiltinIteratorReturn` to `undefined`.

```ds:main.ds libs=es5,es2015.iterable
type ReturnValue = BuiltinIteratorReturn;

const value: ReturnValue = 1;
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{
  "compilerOptions": { "strict": true, "lib": ["es5", "es2015.iterable"] }
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

```ds:dsconfig.json
{ "compilerOptions": { "strict": true } }
```

- contains: property is not definitely assigned

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

```ds:dsconfig.json
{ "compilerOptions": { "strict": true } }
```

- contains: expected string
