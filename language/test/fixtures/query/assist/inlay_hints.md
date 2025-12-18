# Inlay Hints

## Function Calls

### Parameter hints for function calls

Inlay hints should show parameter names at each argument position in a function call.

```ds
function greet(name: string, greeting: string): string {
    return greeting + ", " + name;
}

const msg = greet("World", "Hello");
```

`greet("World", "Hello")` has 2 arguments, so we expect 2 parameter hints plus 1 type hint for `msg`.

```query inlay_hints $0
3
```

### Parameter hints for multi-argument calls

Inlay hints count should match the number of arguments passed to a function.

```ds
function calculate(a: int32, b: int32, c: int32): int32 {
    return a + b + c;
}

const result = calculate(1, 2, 3);
```

`calculate(1, 2, 3)` has 3 arguments, so we expect 3 parameter hints plus 1 type hint for `result`.

```query inlay_hints $0
4
```

## Type Hints

### Type hint for variable with inferred type

Type hints should show inferred types for variables without explicit annotations.

```ds
const x = 42;
const y = "hello";
```

Variables `x` and `y` should get type hints. With 0 function calls, only type hints.

```query inlay_hints $0
2
```

### Type hint for variable with explicit type

Variables with explicit type annotations should not get type hints.

```ds
const x: int32 = 42;
const y: string = "hello";
```

No type hints expected (types already annotated), no function calls.

```query inlay_hints $0
0
```

### Mixed parameter and type hints

Both parameter hints and type hints should be shown together.

```ds
function double(n: int32): int32 {
    return n * 2;
}

const result = double(21);
```

1 parameter hint for `double(21)`, plus 1 type hint for `result`.

```query inlay_hints $0
2
```
