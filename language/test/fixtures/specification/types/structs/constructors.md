# Struct Constructors

## tagged struct literal constructs nominal type

> Tagged struct literals yield the declared struct type.

```ds
struct Counter {
    value: int32

    increment(): Counter {
        Counter { value: this.value + 1 }
    }
}

const counter = Counter { value: 1 };
const next = counter.increment();
next satisfies Counter;
```

## tagged struct literal supports method calls

> Tagged struct literals expose struct methods immediately.

```ds
struct Counter {
    value: int32

    increment(): Counter {
        Counter { value: this.value + 1 }
    }
}

const value = Counter { value: 1 }.increment().value;
value satisfies int32;
```

## untagged object literal is not a struct

> Untagged object literals do not satisfy nominal struct types.

```ds
struct Counter {
    value: int32
}

const counter: Counter = { value: 1 };
```

- contains: is not assignable
