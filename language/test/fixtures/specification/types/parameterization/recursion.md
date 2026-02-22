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

### mutually recursive aliases are rejected

> Mutually recursive aliases should report recursion instead of looping.

```ds
type A<T> = B<T>;
type B<T> = A<T>;

declare let value: A<number>;
```

- contains: recursive

### recursive aliases through mapped projections are rejected

> Recursive aliases in mapped projections should report recursion instead of looping.

```ds
type Remap<T> = { [K in keyof T]: Remap<T[K]> };

declare let value: Remap<{ name: string }>;
```

- contains: recursive

### recursive aliases across modules are rejected

> Cross-module alias cycles should report recursion instead of stalling.

```ds:a.ds
import type { B } from "./b";

export type A<T> = B<T>;
```

```ds:b.ds
import type { A } from "./a";

export type B<T> = A<T>;
```

```ds:main.ds
import type { A } from "./a";

declare let value: A<number>;
```

- contains: recursive
