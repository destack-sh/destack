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
active_signature=0 active_parameter=0
signature[0] label=add(x: int32, y: int32): int32 documentation=<none> parameters=x: int32|y: int32 param_docs=<none>|<none>
```

### Track active parameter by cursor position

Signature help should mark the active parameter based on the cursor location.

```ds
function add(x: int32, y: int32): int32 {
    return x + y;
}

const result = add(1, $0);
```

With the cursor on the second argument, the active parameter should be 1.

```query signature_help $0
active_signature=0 active_parameter=1
signature[0] label=add(x: int32, y: int32): int32 documentation=<none> parameters=x: int32|y: int32 param_docs=<none>|<none>
```

### Track active parameter after trailing comma

Signature help should keep the last parameter active after a trailing comma.

```ds
function add(x: int32, y: int32): int32 {
    return x + y;
}

const result = add(1, 2,$0);
```

```query signature_help $0
active_signature=0 active_parameter=1
signature[0] label=add(x: int32, y: int32): int32 documentation=<none> parameters=x: int32|y: int32 param_docs=<none>|<none>
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
active_signature=0 active_parameter=0
signature[0] label=add(x: int32, y: int32): int32 documentation=<none> parameters=x: int32|y: int32 param_docs=<none>|<none>
```

### Fallback signature for unresolved calls

Signature help should still return a placeholder signature when the call target is unresolved.

```ds
function main() {
    unknownCall(1, 2$0);
}
```

```query signature_help $0
active_signature=0 active_parameter=1
signature[0] label=unknownCall(arg0, arg1) documentation=<none> parameters=arg0|arg1 param_docs=<none>|<none>
```

## Parameter Documentation

### Show @param documentation

Functions with @param documentation should show the parameter docs in signature help.

```ds
/// Formats a greeting message.
/// @param name The name of the person to greet
/// @param formal Whether to use a formal greeting
function greet(name: string, formal: bool): string {
    if formal {
        return "Hello, " + name;
    }
    return "Hi, " + name;
}

const message = greet($0);
```

Signature help should include the function name.

```query signature_help $0
active_signature=0 active_parameter=0
signature[0] label=greet(name: string, formal: <unevaluated>): string documentation=Formats a greeting message. parameters=name: string|formal: <unevaluated> param_docs=The name of the person to greet|Whether to use a formal greeting
```

### Show docs from line comments

Line doc comments should provide parameter documentation.

```ds
/// Compute a sum.
/// @param left The left addend
/// @param right The right addend
function sum(left: int32, right: int32): int32 {
    return left + right;
}

const total = sum($0);
```

```query signature_help $0
active_signature=0 active_parameter=0
signature[0] label=sum(left: int32, right: int32): int32 documentation=Compute a sum. parameters=left: int32|right: int32 param_docs=The left addend|The right addend
```

### Signature help for nested calls

Signature help should target the innermost call at the cursor.

```ds
function add(x: int32, y: int32): int32 {
    return x + y;
}

function mul(x: int32, y: int32): int32 {
    return x * y;
}

const result = add(1, mul($0));
```

```query signature_help $0
active_signature=0 active_parameter=0
signature[0] label=mul(x: int32, y: int32): int32 documentation=<none> parameters=x: int32|y: int32 param_docs=<none>|<none>
```

### Parameter docs for methods

Method doc comments should surface in signature help.

```ds
class Greeter {
    /// Greet someone.
    /// @param name The person to greet
    greet(name: string): string {
        return name;
    }
}

const greeter = Greeter {};
const message = greeter.greet($0);
```

```query signature_help $0
active_signature=0 active_parameter=0
signature[0] label=greet(name: string): string documentation=Greet someone. parameters=name: string param_docs=The person to greet
```
