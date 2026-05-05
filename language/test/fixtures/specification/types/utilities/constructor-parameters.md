# ConstructorParameters

`ConstructorParameters` extracts a constructor parameter tuple.

## cases

### constructorparameters extracts constructor arguments

> Constructor parameters are returned as a tuple.

```ts libs=es5
class User {
    constructor(name: string, age: number) {}
}

type Args = ConstructorParameters<typeof User>;

const ok: Args = ["Ada", 42];
ok satisfies [string, number];
```

### constructorparameters rejects wrong argument types

> Extracted constructor parameters keep each parameter type.

```ts libs=es5
class User {
    constructor(name: string, age: number) {}
}

type Args = ConstructorParameters<typeof User>;

const bad: Args = ["Ada", "old"];
```

- contains: not assignable
