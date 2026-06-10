# Struct Construction

Structs use tagged literals for construction.

## literals

### tagged literals construct structs

`Point { ... }` is the struct constructor.

```ds
struct Counter {
    value: int32;

    increment(): Counter {
        Counter { value: this.value + 1 }
    }
}

const counter = Counter { value: 1 };
const next = counter.increment();
next satisfies Counter;
```

### tagged literals expose methods

Constructed values carry their methods.

```ds
struct Counter {
    value: int32;

    increment(): Counter {
        Counter { value: this.value + 1 }
    }
}

const value = (Counter { value: 1 }).increment().value;
value satisfies int32;
```

### tagged literals require fields

Every field must be initialized.

```ds
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1 };
```

- contains: not assignable

### tagged literals reject extra fields

The field list is closed.

```ds
struct Point {
    x: int32;
    y: int32;
}

const point = Point { x: 1, y: 2, z: 3 };
```

- contains: excess property

## constructors

### structs reject new

`new` is for classes.

```ds
struct Point {
    x: int32;
    y: int32;
}

const point = new Point(1, 2);
```

- contains: construct

### struct methods can mutate fields

Methods mutate through `this` as usual.

```ds
struct Counter {
    value: int32;

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

Structs are nominal; the tag is required.

```ds
struct Counter {
    value: int32;
}

const counter: Counter = { value: 1 };
```

- contains: is not assignable
