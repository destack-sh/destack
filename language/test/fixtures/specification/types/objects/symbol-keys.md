# Symbol Keys

Unique symbols key properties nominally.

## symbol properties

### keyof preserves symbol index signatures

Keyof on a symbol index signature yields symbol.

```ds
interface SymbolBag {
    [key: symbol]: int32;
}

type Keys = keyof SymbolBag;

declare const key: symbol;
const ok: Keys = key;
const bad: Keys = "name";
```

- contains: not assignable

### Symbol.for keys are accepted

Registry symbol keys can be used in object types.

```ds
interface RegistryBox {
    [Symbol.for("token")]: string;
}

const box: RegistryBox = { [Symbol.for("token")]: "ok" };
box[Symbol.for("token")] satisfies string;
```

### unique symbol keys are accepted

Unique symbols can be used as object keys.

```ds
const token: unique symbol = Symbol("token");

interface TokenBox {
    [token]: int32;
}

const box: TokenBox = { [token]: 1 };
box[token] satisfies int32;
```

### keyof preserves unique symbol keys

Keyof preserves the unique symbol key.

```ds
const token: unique symbol = Symbol("token");

interface TokenBox {
    [token]: int32;
}

type Keys = keyof TokenBox;

const ok: Keys = token;
```

### keyof rejects unrelated symbols

Keyof does not accept unrelated symbols.

```ds
const token: unique symbol = Symbol("token");

interface TokenBox {
    [token]: int32;
}

type Keys = keyof TokenBox;

const bad: Keys = Symbol("other");
```

- contains: not assignable
