# Struct Construction

Structs use tagged literals for construction.

## literals

### tagged literals construct structs

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

### tagged literals expose methods

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

### tagged literals require fields

```ds
struct Point {
    x: int32
    y: int32
}

const point = Point { x: 1 };
```

- contains: not assignable

### tagged literals reject extra fields

```ds
struct Point {
    x: int32
    y: int32
}

const point = Point { x: 1, y: 2, z: 3 };
```

- contains: excess property

## constructors

### structs reject new

```ds
struct Point {
    x: int32
    y: int32
}

const point = new Point(1, 2);
```

- contains: construct

### struct methods can mutate fields

```ds
struct Counter {
    value: int32

    increment(): int32 {
        this.value = this.value + 1;
        this.value
    }
}

let counter = Counter { value: 1 };
const next = counter.increment();
next satisfies int32;
```

## objects

### object literals do not construct structs

```ds
struct Counter {
    value: int32
}

const counter: Counter = { value: 1 };
```

- contains: is not assignable
