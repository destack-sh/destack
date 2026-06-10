# Typeof

`typeof` lifts value types into type space.

## value queries

### typeof returns value types for locals

`typeof` lifts a binding's type into type space.

```ds
const value = 42;

type ValueType = typeof value;

let ok: ValueType = 42;
```

### typeof rejects incompatible locals

The lifted type is exact.

```ds
const value = 42;

type ValueType = typeof value;

let bad: ValueType = "no";
```

- contains: not assignable

### typeof returns constructor types for classes

A class value's type is its constructor surface.

```ds
class Counter {
    static version: int32;
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }
}

type CounterCtor = typeof Counter;

declare function takesCounter(ctor: { new (value: int32): Counter }): void;

takesCounter(Counter);

let okVersion: CounterCtor["version"] = 1;
```

### typeof rejects incompatible static fields

Static fields keep their declared types.

```ds
class Counter {
    static version: int32;
    value: int32;

    constructor(value: int32) {
        this.value = value;
    }
}

type CounterCtor = typeof Counter;

let badVersion: CounterCtor["version"] = "no";
```

- contains: not assignable

### typeof includes static methods

Static methods are part of the constructor surface.

```ds
class Counter {
    static next(value: int32): int32 {
        return value + 1;
    }
}

type CounterCtor = typeof Counter;
type CounterNext = (typeof Counter)["next"];

let ctor: CounterCtor = Counter;
let okNext: int32 = ctor.next(1);

let okFn: CounterNext = Counter.next;
```

### typeof rejects static method call assignability

Lifted signatures check like any function type.

```ds
class Counter {
    static next(value: int32): int32 {
        return value + 1;
    }
}

type CounterCtor = typeof Counter;

let ctor: CounterCtor = Counter;
let badNext: string = ctor.next(1);
```

- contains: not assignable
