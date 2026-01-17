# Reference Basics

## immutable references

### shared reference expression yields reference type

> Reference expressions yield `&T` types.

```ds
struct Point {
    x: int32,
}

let point = Point { x: 1 };
let shared: &Point = &point;
```

### shared reference is not assignable to owned type

> References are not assignable to owned values.

```ds
struct Point {
    x: int32,
}

let point = Point { x: 1 };
let value: Point = &point;
```

- contains: not assignable

### shared reference is not assignable to mutable reference

> Shared references cannot be assigned to mutable references.

```ds
struct Point {
    x: int32,
}

let point = Point { x: 1 };
let shared: &Point = &point;
let mutable_ref: &mut Point = shared;
```

- contains: not assignable
