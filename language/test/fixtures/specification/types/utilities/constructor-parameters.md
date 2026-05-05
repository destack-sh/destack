# ConstructorParameters

`ConstructorParameters` extracts a constructor parameter tuple.

### ConstructorParameters extracts constructor arguments

```ds
class User {
    constructor(name: string, age: number) {}
}

type Args = ConstructorParameters<typeof User>;

const ok: Args = ("Ada", 42);
ok satisfies (string, number);
```

### ConstructorParameters rejects wrong argument types

```ds
class User {
    constructor(name: string, age: number) {}
}

type Args = ConstructorParameters<typeof User>;

const bad: Args = ("Ada", "old");
```

- contains: not assignable
