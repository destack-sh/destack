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

```ds:dsconfig.json
{ "compilerOptions": { "noDynamicImport": true } }
```

- contains: dynamic imports are disabled

## noDynamicEvaluation

### noDynamicEvaluation reports eval when true

> Eval calls are rejected when noDynamicEvaluation is true.

```ds:main.ds libs=es2020
let value = Function(["return 1"]);
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noDynamicEvaluation": true } }
```

- contains: dynamic evaluation is disabled

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

```ds:dsconfig.json
{ "compilerOptions": { "noProxy": true } }
```

- contains: proxy usage is disabled

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

```ds:dsconfig.json
{ "compilerOptions": { "noDynamicShapes": true } }
```

- contains: dynamic shape mutation is disabled

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

```ds:dsconfig.json
{ "compilerOptions": { "noManaged": true } }
```

- contains: managed memory is disabled

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

```ds:dsconfig.json
{ "compilerOptions": { "noManaged": true } }
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

```ds:dsconfig.json
{ "compilerOptions": { "noRuntime": true } }
```

- contains: runtime features are disabled

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

```ds:dsconfig.json
{ "compilerOptions": { "noComputedPropertyAccess": true } }
```

- contains: computed property access is disabled

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

```ds:dsconfig.json
{ "compilerOptions": { "noReferentialEquality": true } }
```

- contains: referential equality is disabled

## noImplicitDynamicDispatch

### noImplicitDynamicDispatch reports union dispatch when true

> Implicit dynamic dispatch is rejected when noImplicitDynamicDispatch is true.

```ds:main.ds
struct Cat {
    name: string,

    speak(): string {
        "meow"
    }
}

struct Dog {
    name: string,

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

```ds:dsconfig.json
{ "compilerOptions": { "noImplicitDynamicDispatch": true } }
```

- contains: implicit dynamic dispatch is disabled

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

```ds:dsconfig.json
{ "compilerOptions": { "noGlobalThis": true } }
```

- contains: globalThis access is disabled
