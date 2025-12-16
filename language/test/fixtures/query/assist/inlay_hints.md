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

`greet("World", "Hello")` has 2 arguments, so we expect 2 parameter hints.

```query inlay_hints $0
2
```

### Parameter hints for multi-argument calls

Inlay hints count should match the number of arguments passed to a function.

```ds
function calculate(a: int32, b: int32, c: int32): int32 {
    return a + b + c;
}

const result = calculate(1, 2, 3);
```

`calculate(1, 2, 3)` has 3 arguments, so we expect 3 parameter hints.

```query inlay_hints $0
3
```
