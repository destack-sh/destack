# Typeof

## value queries

### typeof returns value types for locals

> `typeof` returns the value type of a local binding.

```ds
const value = 42;

type ValueType = typeof value;

let ok: ValueType = 42;
```

### typeof rejects incompatible locals

> `typeof` preserves the local binding type.

```ds
const value = 42;

type ValueType = typeof value;

let bad: ValueType = "no";
```

- contains: not assignable

### typeof returns constructor types for classes

> `typeof` on a class returns the constructor value type with static members.

```ds
class Counter {
    static version: int32
    value: int32

    constructor(value: int32) {
        this.value = value;
    }
}

type CounterCtor = typeof Counter;

declare function takesCounter(ctor: { new(value: int32): Counter }): void;

takesCounter(Counter);

let okVersion: CounterCtor["version"] = 1;
```

### typeof rejects incompatible static fields

> `typeof` preserves static field types.

```ds
class Counter {
    static version: int32
    value: int32

    constructor(value: int32) {
        this.value = value;
    }
}

type CounterCtor = typeof Counter;

let badVersion: CounterCtor["version"] = "no";
```

- contains: not assignable

### typeof includes static methods

> `typeof` exposes static methods on the constructor value type.

```ds
class Counter {
    static next(value: int32): int32 { return value + 1 }
}

type CounterCtor = typeof Counter;
type CounterNext = (typeof Counter)["next"];

let ctor: CounterCtor = Counter;
let okNext: int32 = ctor.next(1);

let okFn: CounterNext = Counter.next;
```

### typeof rejects static method call assignability

> `typeof` static method results must match the expected type.

```ds
class Counter {
    static next(value: int32): int32 { return value + 1 }
}

type CounterCtor = typeof Counter;

let ctor: CounterCtor = Counter;
let badNext: string = ctor.next(1);
```

- contains: not assignable
