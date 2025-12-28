# Index Signatures

## string index signature accepts matching fields

> String index signatures allow string and numeric keys with compatible values.

```ds
interface Bag {
    [key: string]: number
}

const bag = { a: 1, 2: 3 }
bag satisfies Bag;
```

## string index signature rejects incompatible fields

> Fields with incompatible value types are rejected.

```ds
interface Bag {
    [key: string]: number
}

const bag = { a: 1, b: "two" }
bag satisfies Bag;
```

- contains: not assignable

## satisfies preserves literal type for index access

> `satisfies` does not widen object literals for index access.

```ds
interface Bag {
    [key: string]: number
}

const bag = { a: 1 } satisfies Bag
let value: number = bag["a"]
```

## satisfies does not add index signature

> Missing properties still reject index access after `satisfies`.

```ds
interface Bag {
    [key: string]: number
}

const bag = { a: 1 } satisfies Bag
let value: number = bag["missing"]
```

- contains: indexing non-indexable

## number index signature rejects string fields

> Number index signatures do not accept string fields.

```ds
interface NumberBag {
    [key: number]: number
}

const bag = { a: 1 }
bag satisfies NumberBag;
```

- contains: not assignable

## number index signature accepts numeric fields

> Number index signatures accept numeric field keys.

```ds
interface NumberBag {
    [key: number]: string
}

const bag = { 1: "one", 2: "two" }
bag satisfies NumberBag;
```

## noPropertyAccessFromIndexSignature forbids dot access

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

## dot access from index signature is allowed by default

```ds
interface Bag {
    [key: string]: number
}

const bag: Bag = { a: 1 }
let value = bag.missing
```
