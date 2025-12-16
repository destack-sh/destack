# Signature Help

## Function Calls

### Show signature for function call

When the cursor is inside a function call's argument list, signature help should display the function name.

```ds
function add(x: int32, y: int32): int32 {
    return x + y;
}

const result = add($0);
```

With the cursor at `add($0)`, signature help should show "add".

```query signature_help $0
add
```

### Show signature for method call

When the cursor is inside a method call's argument list, signature help should display the method name.

```ds
struct Calculator {
}

extension for Calculator {
    add(x: int32, y: int32): int32 {
        return x + y;
    }
}

const calc = Calculator {};
const result = calc.add($0);
```

For `calc.add($0)`, a method call on a Calculator instance, signature help should show "add".

```query signature_help $0
add
```
