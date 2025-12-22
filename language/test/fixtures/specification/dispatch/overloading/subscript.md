# Subscript Operator Overloading

Tests for subscript operator overloading via interface implementations.

## Index access

### index access dispatches to Index

> Index access uses the Index interface when implemented.

```ds
struct Bag { value: int }

extension for Bag implements Index<int, int> {
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

extension for Bag implements IndexSet<int, int> {
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

extension for Bag implements IndexSet<int, int> {
    indexSet(key: int, value: int): void {}
}

declare function getBag(): Bag;

const bag = getBag();
bag[1] = "nope";
```

- contains: not assignable
