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

```ds:main.ds
declare const Function: (source: string) => unknown;

let value = Function("return 1");
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

```ds:main.ds
declare const Proxy: { new (target: object, handler: object): object };

let handler = {};
let target = {};
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

```ds:main.ds
declare const Object: {
    defineProperty(target: object, key: string, descriptor: object): void;
};

let target = { value: 1 };
Object.defineProperty(target, "x", { value: 1 });
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noDynamicShapes": true } }
```

- contains: dynamic shape mutation is disabled

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
