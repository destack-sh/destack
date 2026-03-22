# Struct Constructors

## tagged struct literals

### tagged struct literal constructs nominal type

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

### tagged struct literal supports method calls

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

### tagged struct literal rejects missing fields

> Tagged struct literals must provide required fields.

```ds
struct Point {
    x: int32
    y: int32
}

const point = Point { x: 1 };
```

- not assignable

### tagged struct literal rejects extra fields

> Tagged struct literals reject excess fields.

```ds
struct Point {
    x: int32
    y: int32
}

const point = Point { x: 1, y: 2, z: 3 };
```

- excess property

## struct constructors

### new struct constructor uses positional fields

> Positional constructors follow field order.

```ds
struct Point {
    x: int32
    y: int32
}

const point = new Point(1, 2);
point satisfies Point;
```

### new struct constructor rejects mismatched argument types

> Positional constructors enforce field types.

```ds
struct Point {
    x: int32
    y: int32
}

const point = new Point("a", 2);
```

- not assignable

### new struct constructor rejects missing arguments

> Positional constructors require all fields.

```ds
struct Point {
    x: int32
    y: int32
}

const point = new Point(1);
```

- no matching overload

### new struct constructor rejects extra arguments

> Positional constructors reject extra arguments.

```ds
struct Point {
    x: int32
    y: int32
}

const point = new Point(1, 2, 3);
```

- no matching overload

### struct methods can mutate this

> Struct methods can update fields through `this`.

```ds
struct Counter {
    value: int32

    increment(): int32 {
        this.value = this.value + 1;
        this.value
    }
}

const counter = Counter { value: 1 };
const next = counter.increment();
next satisfies int32;
```

## untagged object literals

### untagged object literal is not a struct

> Untagged object literals do not satisfy nominal struct types.

```ds
struct Counter {
    value: int32
}

const counter: Counter = { value: 1 };
```

- is not assignable
