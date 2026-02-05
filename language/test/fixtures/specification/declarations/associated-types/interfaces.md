# Associated Types: Interfaces

Interface associated type tests live here.

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

### interface associated type can be generic

> Interfaces can declare generic associated types.

```ds
interface Slice<T> {
    type View<U>;
}
```

### interface associated type can use static value parameters

> Interface associated types can declare comptime static value parameters.

```ds
interface Windowed<T> {
    type View<comptime n: uint>;
}
```

### interface associated type default can be used by implementors

> Implementors can rely on interface associated type defaults.

```ds
interface Iterable<T> {
    type Item = T;
    next(): Item;
}

struct Counter {
    value: int32 = 0;
}

extension for Counter implements Iterable<int32> {
    next(): Item {
        this.value
    }
}

declare const counter: Counter;
counter.next() satisfies int32;
```

### interface associated type default can be overridden

> Implementors can override interface associated type defaults.

```ds
interface Wrapper<T> {
    type Item = T;
}

struct Box<T> {
    value: T;
}

extension<T> for Box<T> implements Wrapper<T> {
    type Item = [T, T];
}

declare const value: Box<int32>.Item;
value satisfies [int32, int32];
```

### interface associated type rejects incompatible implementations

> Implementor associated types must satisfy interface constraints.

```ds
interface SizedIterable<T> {
    type Item: number;
    next(): Item;
}

struct Bad {
    value: string = "no";
}

extension for Bad implements SizedIterable<int32> {
    type Item = string;

    next(): Item {
        this.value
    }
}
```

- contains: not assignable

### interface associated type requires matching parameter arity

> Implementor associated types must match the required parameter arity.

```ds
interface Factory {
    type Item<T>;
}

struct Thing {}

extension for Thing implements Factory {
    type Item = int32;
}
```

- contains: parameter

### interface associated type requires matching parameter kinds

> Implementors must match parameter kinds for associated types.

```ds
interface Windowed<T> {
    type View<comptime n: uint>;
}

struct Samples {
    value: int32 = 0;
}

extension for Samples implements Windowed<int32> {
    type View<T> = [T, T];
}
```

- contains: parameter

### associated type projection on constrained type parameters

> Projections are allowed on constrained type parameters.

```ds
interface Iterable<T> {
    type Item;
}

function head<I: Iterable<int32>>(value: I.Item): I.Item {
    return value;
}
```

### associated type projection requires static value arguments

> Projections must supply required static value arguments.

```ds
interface Windowed<T> {
    type View<comptime n: uint>;
}

function take<W: Windowed<int32>>(value: W.View): W.View {
    return value;
}
```

- contains: argument
