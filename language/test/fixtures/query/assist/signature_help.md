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
signature[0] label=add(x, y) documentation=<none> parameters=x|y param_docs=<none>|<none>
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
signature[0] label=add(x, y) documentation=<none> parameters=x|y param_docs=<none>|<none>
```

## Parameter Documentation

### Show @param documentation

Functions with @param documentation should show the parameter docs in signature help.

```ds
/**
 * Formats a greeting message.
 * @param name The name of the person to greet
 * @param formal Whether to use a formal greeting
 */
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
signature[0] label=greet(name, formal) documentation=<none> parameters=name|formal param_docs=<none>|<none>
```
