# Index Signatures

## string index signatures

### string index signature accepts matching fields

> String index signatures allow string and numeric keys with compatible values.

```ds
interface Bag {
    [key: string]: number
}

const bag = { a: 1, 2: 3 }
bag satisfies Bag;
```

### string index signature rejects incompatible fields

> Fields with incompatible value types are rejected.

```ds
interface Bag {
    [key: string]: number
}

const bag = { a: 1, b: "two" }
bag satisfies Bag;
```

- contains: expected Bag

### satisfies preserves literal type for index access

> `satisfies` does not widen object literals for index access.

```ds
interface Bag {
    [key: string]: number
}

const bag = { a: 1 } satisfies Bag
let value: number = bag["a"]
```

### satisfies does not add index signature

> Missing properties still reject index access after `satisfies`.

```ds
interface Bag {
    [key: string]: number
}

const bag = { a: 1 } satisfies Bag
let value = bag["missing"]
```

- contains: indexing non-indexable

## number index signatures

### number index signature allows string fields

> Number index signatures allow string fields.

```ds
interface NumberBag {
    [key: number]: number
}

const bag = { a: 1 }
bag satisfies NumberBag;
```

### number index signature accepts numeric fields

> Number index signatures accept numeric field keys.

```ds
interface NumberBag {
    [key: number]: string
}

const bag = { 1: "one", 2: "two" }
bag satisfies NumberBag;
```

### number index signature accepts numeric string index access

> Numeric string literals index number index signatures.

```ds
interface NumberBag {
    [key: number]: string
}

const bag: NumberBag = { 1: "one", 2: "two" }
let value: string | undefined = bag["1"]
```

### number index signature rejects non numeric string index access

> Non numeric string literals do not index number index signatures.

```ds
interface NumberBag {
    [key: number]: string
}

const bag: NumberBag = { 1: "one" }
let value = bag["missing"]
```

- contains: indexing non-indexable

## noUncheckedIndexedAccess

### noUncheckedIndexedAccess adds undefined to index access

> Index signature access includes undefined when noUncheckedIndexedAccess is true.

```ds:main.ds
interface Bag {
    [key: string]: int32
}

const bag: Bag = { a: 1 }
let value: int32 = bag["a"]
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noUncheckedIndexedAccess": true } }
```

- contains: not assignable

### noUncheckedIndexedAccess leaves index access unchanged when false

> Index signature access keeps the value type when noUncheckedIndexedAccess is false.

```ds:main.ds
interface Bag {
    [key: string]: int32
}

const bag: Bag = { a: 1 }
let value: int32 = bag["a"]
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noUncheckedIndexedAccess": false } }
```

## property access

### noPropertyAccessFromIndexSignature forbids dot access

```ds:dsconfig.json
{ "compilerOptions": { "noPropertyAccessFromIndexSignature": true } }
```

```ds:package.json
{ "name": "spec" }
```

```ds
interface Bag {
    [key: string]: number
}

const bag: Bag = { a: 1 }
let value = bag.missing
```

- contains: only available via index signature

### noPropertyAccessFromIndexSignature allows dot access when false

```ds:main.ds
interface Bag {
    [key: string]: number
}

const bag: Bag = { a: 1 }
let value = bag.missing
```

```ds:package.json
{ "name": "spec" }
```

```ds:dsconfig.json
{ "compilerOptions": { "noPropertyAccessFromIndexSignature": false } }
```

### dot access from index signature is allowed by default

```ts:main.ts
interface Bag {
    [key: string]: number
}

const bag: Bag = { a: 1 }
let value = bag.missing
```
