# Subscript Operator Overloading

Tests for subscript operator overloading via interface implementations.

## Index access

### index access dispatches to Index

> Index access uses the Index interface when implemented.

```ds
struct Bag { value: int }

extension of Bag implements Index<int, int> {
    index(key: int): int { return key }
}

declare function getBag(): Bag;

const bag = getBag();
const value = bag[1];
value satisfies int;
```

## Index assignment

### index assignment dispatches to IndexSet

> Index assignment uses the IndexSet interface when implemented.

```ds
struct Bag { value: int }

extension of Bag implements IndexSet<int, int> {
    indexSet(key: int, value: int): void {}
}

declare function getBag(): Bag;

const bag = getBag();
bag[1] = 2;
```

### index assignment checks value type

> Index assignment requires the value to match the index set parameter type.

```ds
struct Bag { value: int }

extension of Bag implements IndexSet<int, int> {
    indexSet(key: int, value: int): void {}
}

declare function getBag(): Bag;

const bag = getBag();
bag[1] = "nope";
```

- type unknown is not assignable to type int32

### index access rejects missing Index contracts

> Index access requires an Index contract implementation.

```ds
struct Bag { value: int }

declare function getBag(): Bag;

const bag = getBag();
const value = bag[1];
value satisfies int;
```

- contains: no matching overload

### index assignment rejects missing IndexSet contracts

> Index assignment requires an IndexSet contract implementation.

```ds
struct Bag { value: int }

declare function getBag(): Bag;

const bag = getBag();
bag[1] = 2;
```

- contains: no matching overload

### index access checks key type compatibility

> Index access should reject keys that do not match the index contract key type.

```ds
struct Bag { value: int }

extension of Bag implements Index<int, int> {
    index(key: int): int { return key }
}

declare function getBag(): Bag;

const bag = getBag();
const value = bag["one"];
value satisfies int;
```

- type unknown is not assignable to type int32