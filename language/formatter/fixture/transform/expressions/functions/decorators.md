# Function Decorators

## Decorators

### decorated function

Function decorators appear on their own line above the function.

```tspp
@deprecated("use newFoo")
function oldFoo() { }
```

```tspp expected
@deprecated("use newFoo")
function oldFoo() {}
```

### multiple decorators

Multiple decorators each get their own line, in order.

```tspp
@log
@memoize
function compute(x: number): number { return x * 2 }
```

```tspp expected
@log
@memoize
function compute(x: number): number {
    return x * 2;
}
```

### decorator with arguments

Decorator arguments follow function call formatting rules.

```tspp
@route("/api/users", { method: "GET" })
async function getUsers() { }
```

```tspp expected
@route("/api/users", { method: "GET" })
async function getUsers() {}
```
