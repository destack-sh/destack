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
