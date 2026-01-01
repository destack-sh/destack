# Symbol Keys

## Symbol.iterator keys are accepted

> Well-known symbol keys can be used in object types.

```ds:dsconfig.json
{ "compilerOptions": { "lib": ["es2015"] } }
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

## Symbol.for keys are accepted

> Registry symbol keys can be used in object types.

```ds:dsconfig.json
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

## unique symbol keys are accepted

> Unique symbols can be used as object keys.

```ds:dsconfig.json
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
