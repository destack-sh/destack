# Symbol Keys

## tests

### Symbol.iterator keys are accepted

> Well-known symbol keys can be used in object types.

```json:destack.json
{ "compilerOptions": { "lib": ["es2015"], "noUncheckedIndexedAccess": false } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
interface IterableBox {
    [Symbol.iterator]: int32
}

const box: IterableBox = { [Symbol.iterator]: 1 }
box[Symbol.iterator] satisfies int32
```

### symbol index signatures are accepted

> Symbol index signatures accept symbol keys.

```json:destack.json
{ "compilerOptions": { "lib": ["es2015"], "noUncheckedIndexedAccess": false } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
interface SymbolBag {
    [key: symbol]: int32
}

const token: unique symbol = Symbol("token")
const box: SymbolBag = { [token]: 1 }
box[token] satisfies int32
```

### keyof preserves symbol index signatures

> Keyof on a symbol index signature yields symbol.

```json:destack.json
{ "compilerOptions": { "lib": ["es2015"] } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
interface SymbolBag {
    [key: symbol]: int32
}

type Keys = keyof SymbolBag;

declare const key: symbol;
const ok: Keys = key;
const bad: Keys = "name";
```

- contains: not assignable

### keyof preserves well-known symbol keys

> Keyof should include the specific well-known symbol.

```json:destack.json
{ "compilerOptions": { "lib": ["es2015"] } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
interface IterableBox {
    [Symbol.iterator]: int32
    name: string
}

type Keys = keyof IterableBox;

const goodWellKnown: Keys = Symbol.iterator;
const goodName: Keys = "name";
const other: symbol = Symbol("other");
const badSymbol: Keys = other;
```

- contains: not assignable

### Symbol.for keys are accepted

> Registry symbol keys can be used in object types.

```json:destack.json
{ "compilerOptions": { "lib": ["es2015"] } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
interface RegistryBox {
    [Symbol.for("token")]: string
}

const box: RegistryBox = { [Symbol.for("token")]: "ok" }
box[Symbol.for("token")] satisfies string
```

### unique symbol keys are accepted

> Unique symbols can be used as object keys.

```json:destack.json
{ "compilerOptions": { "lib": ["es2015"] } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
const token: unique symbol = Symbol("token")

interface TokenBox {
    [token]: int32
}

const box: TokenBox = { [token]: 1 }
box[token] satisfies int32
```

### keyof preserves unique symbol keys

> Keyof preserves the unique symbol key.

```json:destack.json
{ "compilerOptions": { "lib": ["es2015"] } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
const token: unique symbol = Symbol("token")

interface TokenBox {
    [token]: int32
}

type Keys = keyof TokenBox;

const ok: Keys = token;
```

### keyof rejects unrelated symbols

> Keyof should not accept unrelated symbols.

```json:destack.json
{ "compilerOptions": { "lib": ["es2015"] } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
const token: unique symbol = Symbol("token")

interface TokenBox {
    [token]: int32
}

type Keys = keyof TokenBox;

const bad: Keys = Symbol("other");
```

- contains: not assignable
