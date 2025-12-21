# Inference

Tests for parameter and return type inference.

## call arguments infer parameters

> Parameters are inferred from call arguments.

```ds
function add(a, b) {
    return a + b
}
add(1, 2)
add satisfies (a: 1, b: 2) => number;
```

## default values infer parameters

> Parameters are inferred from default values.

```ds
function greet(name = "hi") {
    return name
}
greet satisfies (name: "hi") => "hi";
```

## contextual typing

### contextual lambda from annotation

> Lambda parameter types are inferred from annotations.

```ds
const add: (a: number, b: number) => number = (a, b) => a + b
add satisfies (a: number, b: number) => number;
```

### contextual lambda from argument

> Lambda parameter types are inferred from parameter types.

```ds
function apply(transform: (value: number) => number) {
    return transform(1)
}
apply((value) => value + 1)
```

### contextual object argument

> Object literals use parameter types for contextual typing.

```ds
function use_point(point: { x: number, y: number }) {
    return point.x
}
use_point({ x: 1, y: 2 })
```

### contextual tuple argument

> Tuple literals use parameter types for contextual typing.

```ds
function sum(pair: (number, number)) {
    return pair
}
sum((1, 2))
```

### contextual array argument

> Array literals use parameter types for contextual typing.

```ds
function total(values: number[]) {
    return values
}
total([1, 2, 3])
```
