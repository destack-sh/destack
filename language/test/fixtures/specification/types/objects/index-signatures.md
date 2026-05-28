# Index Signatures

Index signatures describe bracket access on object-shaped types.
They are structural constraints.

## readonly views

### readonly string index signature accepts matching fields

Readonly index signatures can view finite object fields.

```ds
type Bag = { readonly [key: string]: int32 };

const point = { x: 1, y: 2 };
const bag: Bag = point;

bag["x"] satisfies int32 | undefined;
```

### readonly string index signature rejects incompatible fields

Every visible string field must fit the indexed value type.

```ds
type Bag = { readonly [key: string]: int32 };

const mixed = { x: 1, y: "two" };
const bag: Bag = mixed;
```

- contains: not assignable

### satisfies does not add an index signature

`satisfies` checks compatibility without widening the source type.

```ds
type Bag = { readonly [key: string]: int32 };

const point = { x: 1 } satisfies Bag;
const missing = point["missing"];
```

- contains: indexing non-indexable

## writable access

### writable index signature rejects finite objects

Finite object shapes do not provide open writable index access.

```ds
type Bag = { [key: string]: int32 };

const point = { x: 1, y: 2 };
const bag: Bag = point;
```

- contains: not assignable

### writable index signature accepts maps

Maps provide indexed reads and writes.

```ds
type Bag = { [key: string]: int32 };

declare const map: Map<string, int32>;
const bag: Bag = map;

bag["x"] = 1;
bag["x"] satisfies int32 | undefined;
```

### writable index signature accepts operator implementations

Concrete types can satisfy mutable index signatures through `Index` and `IndexSet`.

```ds
struct Bag {
    storage: Map<string, int32>;
}

extension of Bag implements Index<string>, IndexSet<string, int32> {
    type Output = int32 | undefined;

    index(key: string): this.Output {
        this.storage[key]
    }

    indexSet(this: &exclusive Bag, key: string, value: int32): void {
        this.storage[key] = value;
    }
}

declare let bag: Bag;

bag["x"] = 1;
bag satisfies { [key: string]: int32 };
```

## `Record`

### record builds finite required fields

Finite `Record` keys remain a mapped object shape.

```ds
type Flags = Record<"a" | "b", boolean>;

const flags: Flags = { a: true, b: false };
flags["a"] satisfies boolean;
```

### record with string keys requires writable index access

A broad string `Record` cannot be satisfied by a finite object shape.

```ds
type Bag = Record<string, int32>;

const point = { x: 1 };
const bag: Bag = point;
```

- contains: not assignable

## property access

### index signatures reject dot access

Unknown keys must use bracket access.

```ds
type Bag = { readonly [key: string]: int32 };

declare const bag: Bag;
const value = bag.missing;
```

- contains: only available via index signature
