# ConstructorParameters

`ConstructorParameters` extracts a constructor parameter tuple.

## constructors

### ConstructorParameters extracts constructor arguments

The constructor's parameter list becomes a tuple.

```ds
class User {
    constructor(name: string, age: number) {}
}

type Args = ConstructorParameters<typeof User>;

const ok: Args = ("Ada", 42);
ok satisfies (string, number);
```

### ConstructorParameters rejects wrong argument types

The tuple is exact.

```ds
class User {
    constructor(name: string, age: number) {}
}

type Args = ConstructorParameters<typeof User>;

const bad: Args = ("Ada", "old");
```

- contains: not assignable
