# Mutability Basics

## mutable references

### mutable reference expression yields mutable reference type

> Mutable reference expressions yield `&mut T` types.

```ds
struct Point {
    x: int32,
}

let point = Point { x: 1 };
let shared: &mut Point = &mut point;
```

## mutable values

### mutable value expression yields mutable value type

> Mutable value expressions yield `^var T` types.

```ds
struct Point {
    x: int32,
}

let owned: ^var Point = ^var Point { x: 1 };
```
