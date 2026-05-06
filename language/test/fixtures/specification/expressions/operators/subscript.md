# Subscript

Subscript operators use builtin indexed access and receiver interfaces for overloads.

## access

### index access dispatches to Index

Index access uses the Index interface when implemented.

```ds
struct Bag { value: int }

extension of Bag implements Index<int> {
    type Output = int;

    index(key: int): this.Output { return key }
}

declare function getBag(): Bag;

const bag = getBag();
const value = bag[1];
value satisfies int;
```

## assignment

### index assignment dispatches to IndexSet

Index assignment uses the IndexSet interface when implemented.

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

Index assignment requires the value to match the index set parameter type.

```ds
struct Bag { value: int }

extension of Bag implements IndexSet<int, int> {
    indexSet(key: int, value: int): void {}
}

declare function getBag(): Bag;

const bag = getBag();
bag[1] = "nope";
```

- contains: not assignable

### index access rejects missing Index

Index access requires an `Index` implementation.

```ds
struct Bag { value: int }

declare function getBag(): Bag;

const bag = getBag();
const value = bag[1];
value satisfies int;
```

- contains: no matching overload

### index assignment rejects missing IndexSet

Index assignment requires an `IndexSet` implementation.

```ds
struct Bag { value: int }

declare function getBag(): Bag;

const bag = getBag();
bag[1] = 2;
```

- contains: no matching overload

### index access checks key type rules

Index access rejects keys that do not match the index key type.

```ds
struct Bag { value: int }

extension of Bag implements Index<int> {
    type Output = int;

    index(key: int): this.Output { return key }
}

declare function getBag(): Bag;

const bag = getBag();
const value = bag["one"];
value satisfies int;
```

- contains: not assignable
