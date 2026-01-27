# Recursive Type Instantiation

Recursive type instantiation should not crash the compiler.
Recursive instantiation should produce a clear error when it does not converge.

## Recursive aliases

### direct recursive aliases are rejected

> Direct recursive aliases should be rejected with a recursion error.

```ds
type Loop<T> = Loop<T>;

declare let value: Loop<number>;
```

- contains: recursive

## Recursive conditionals

### recursive conditionals report depth errors

> Recursive conditional instantiation should report a recursion or depth error rather than crashing.

```ds
type Recurse<T> = T extends string ? Recurse<T> : never;

declare let value: Recurse<"x">;
```

- contains: recursive

