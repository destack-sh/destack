# Index Signatures

## string index signatures

### string index signature accepts matching fields

String index signatures allow string and numeric keys with compatible values.

```ds
interface Bag {
    [key: string]: number;
}

const bag = { a: 1, 2: 3 };
bag satisfies Bag;
```

### string index signature rejects incompatible fields

Fields with incompatible value types are rejected.

```ds
interface Bag {
    [key: string]: number;
}

const bag = { a: 1, b: "two" };
bag satisfies Bag;
```

- contains: not assignable

### satisfies preserves literal type for index access

`satisfies` does not widen object literals for index access.

```ds
interface Bag {
    [key: string]: number;
}

const bag = { a: 1 } satisfies Bag;
let value: number = bag["a"];
```

### satisfies does not add index signature

Missing properties still reject index access after `satisfies`.

```ds
interface Bag {
    [key: string]: number;
}

const bag = { a: 1 } satisfies Bag;
let value = bag["missing"];
```

- contains: indexing non-indexable

## number index signatures

### number index signature allows string fields

Number index signatures allow string fields.

```ds
interface NumberBag {
    [key: number]: number;
}

const bag = { a: 1 };
bag satisfies NumberBag;
```

### number index signature accepts numeric fields

Number index signatures accept numeric field keys.

```ds
interface NumberBag {
    [key: number]: string;
}

const bag = { 1: "one", 2: "two" };
bag satisfies NumberBag;
```

### number index signature accepts numeric string index access

Numeric string literals index number index signatures.

```ds
interface NumberBag {
    [key: number]: string;
}

const bag: NumberBag = { 1: "one", 2: "two" };
let value: string | undefined = bag["1"];
```

### number index signature rejects non numeric string index access

Non numeric string literals do not index number index signatures.

```ds
interface NumberBag {
    [key: number]: string;
}

const bag: NumberBag = { 1: "one" };
let value = bag["missing"];
```

- contains: indexing non-indexable

## record-like assignability

### record-like assignment accepts object literals

Object literals assignable to index signatures are allowed.

```ds
type Bag = { [key: string]: int32 };

let bag: Bag = { alpha: 1, beta: 2 };
```

### record-like assignment accepts structural objects

Structural object types without index signatures are assignable when fields match.

```ds
type Bag = { [key: string]: int32 };
type Point = { x: int32; y: int32 };

let point: Point = { x: 1, y: 2 };
let bag: Bag = point;
```

### record-like assignment rejects incompatible field values

Fields with incompatible value types are rejected.

```ds
type Bag = { [key: string]: int32 };
type Mixed = { x: int32; y: string };

let mixed: Mixed = { x: 1, y: "two" };
let bag: Bag = mixed;
```

- contains: not assignable

## indexed access

### index signature reads include undefined

Index signature access includes undefined because the key may be absent.

```ds:main.ds
interface Bag {
    [key: string]: int32;
}

const bag: Bag = { a: 1 };
let value: int32 = bag["a"];
```

- contains: not assignable

## property access

### index signatures reject dot access

```ds
interface Bag {
    [key: string]: number;
}

const bag: Bag = { a: 1 };
let value = bag.missing;
```

- contains: only available via index signature
