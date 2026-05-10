# Typeof

## value queries

### typeof returns value types for locals

```ds
const value = 42;

type ValueType = typeof value;

let ok: ValueType = 42;
```

### typeof rejects incompatible locals

```ds
const value = 42;

type ValueType = typeof value;

let bad: ValueType = "no";
```

- contains: not assignable

### typeof returns constructor types for classes

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
