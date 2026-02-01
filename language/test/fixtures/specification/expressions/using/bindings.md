# Using Bindings

## using bindings

### using binding introduces a name

> using bindings introduce a name in the surrounding scope.

```ds
using value = 1;
value satisfies int32;
```

### using expression yields void

> using expressions evaluate to void.

```ds
let result: void = using value = 1;
result satisfies void;
```

### declare using is invalid

> Declare bindings cannot have initializers.

```ts
declare using value = 1;
```

- contains: declare bindings cannot have initializers

## await using bindings

### await using is allowed in async functions

> await using can appear in async scopes.

```ds
async function run(): void {
    await using value = 1;
    value satisfies int32;
}
```
