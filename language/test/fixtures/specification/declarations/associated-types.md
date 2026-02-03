# Associated Types

Associated types are static type members on class shaped declarations.

## structs

### struct associated type alias is allowed

> Structs can declare associated type aliases.

```ds
struct Box<T> {
    type Item = T;
    value: Item;
}

const box = Box { value: "ok" };
box.value satisfies string;
```

### struct associated type can include a constraint

> Associated type aliases can include constraints.

```ds
struct SizedBox {
    type Item: number = int32;
    value: Item;
}

const boxed = SizedBox { value: 1 };
boxed.value satisfies int32;
```

### struct associated type can be referenced from the type

> Associated types are accessed via the containing type.

```ds
struct Box<T> {
    type Item = T;
    value: Item;
}

const value: Box<string>.Item = "ok";
value satisfies string;
```

## classes

### class associated type alias is allowed

> Classes can declare associated type aliases.

```ds
class Box<T> {
    type Item = T;
    value: Item;

    constructor(value: Item) {
        this.value = value;
    }
}

const box = new Box("ok");
box.value satisfies string;
```

## interfaces

### interface associated type is allowed

> Interfaces can declare abstract associated types.

```ds
interface Iterable<T> {
    type Item;
    next(): Item;
}
```

### interface associated type can include a constraint

> Interface associated types can declare a constraint.

```ds
interface SizedIterable {
    type Item: number;
    next(): Item;
}
```

### interface associated type is provided by implementors

> Implementors provide concrete associated types.

```ds
interface Iterable<T> {
    type Item;
    next(): Item;
}

struct Counter {
    value: int32 = 0;
}

extension for Counter implements Iterable<int32> {
    type Item = int32;

    next(): Item {
        this.value
    }
}

declare const counter: Counter;
counter.next() satisfies int32;
```
