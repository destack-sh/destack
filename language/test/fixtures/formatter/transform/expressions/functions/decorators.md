# Function Decorators

## Decorators

### decorated function

Function decorators appear on their own line above the function.

```ds
@deprecated("use newFoo")
function oldFoo() { }
```

```ds expected
@deprecated("use newFoo")
function oldFoo() {}
```

### multiple decorators

Multiple decorators each get their own line, in order.

```ds
@log
@memoize
function compute(x: number): number { return x * 2 }
```

```ds expected
@log
@memoize
function compute(x: number): number {
    return x * 2;
}
```

### decorator with arguments

Decorator arguments follow function call formatting rules.

```ds
@route("/api/users", { method: "GET" })
async function getUsers() { }
```

```ds expected
@route("/api/users", { method: "GET" })
async function getUsers() {}
```
