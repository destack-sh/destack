# ConstructorParameters

`ConstructorParameters` extracts a constructor parameter tuple.

### ConstructorParameters extracts constructor arguments

```ts libs=es5
class User {
    constructor(name: string, age: number) {}
}

type Args = ConstructorParameters<typeof User>;

const ok: Args = ["Ada", 42];
ok satisfies [string, number];
```

### ConstructorParameters rejects wrong argument types

```ts libs=es5
class User {
    constructor(name: string, age: number) {}
}

type Args = ConstructorParameters<typeof User>;

const bad: Args = ["Ada", "old"];
```

- contains: not assignable
