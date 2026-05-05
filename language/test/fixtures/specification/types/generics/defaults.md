# Generic Defaults And Partial Inference

## default chains

### trailing defaults compose from inferred prefixes

> Defaults for trailing type parameters resolve from concrete inferences established earlier in the list.

```ds
declare function triple<T, U = T, V = readonly U[]>(first: T, second?: U, third?: V): (T, U, V);

const value = triple(1);
value[0] satisfies number;
value[1] satisfies number;
value[2][0] satisfies number;
```

### provided middle arguments override default chains

> Supplying an explicit middle argument overrides that link while later defaults keep chaining from the new value.

```ds
declare function triple<T, U = T, V = readonly U[]>(first: T, second?: U, third?: V): (T, U, V);

const value = triple(1, "ok");
value[0] satisfies number;
value[1] satisfies string;
value[2][0] satisfies string;
```

### explicit trailing arguments must satisfy chained defaults

> Even with explicit trailing arguments, constraints implied by earlier inferred defaults are still enforced.

```ds
declare function triple<T, U = T, V = readonly U[]>(first: T, second?: U, third?: V): (T, U, V);

triple(1, "ok", [1]);
```

- contains: not assignable

## keyof defaults

### keyof defaults apply from inferred object parameters

> `keyof` defaults are computed from the inferred object argument and produce the corresponding key union.

```ds
declare function read<T extends { a: number; b: string }, K extends keyof T = keyof T>(value: T, key?: K): T[K];

const value = read({ a: 1, b: "x" });
value satisfies number | string;
```

### provided keys narrow defaulted keyed reads

> Providing a concrete key argument narrows indexed access below the broader default key union.

```ds
declare function read<T extends { a: number; b: string }, K extends keyof T = keyof T>(value: T, key?: K): T[K];

const value = read({ a: 1, b: "x" }, "a");
value satisfies number;
```

### explicit generic prefixes still allow trailing default inference

> Explicitly fixing early generics still allows later parameters to resolve through their declared defaults.

```ds
declare function choose<T, U = T, V = U>(value: T, next?: U, last?: V): (T, U, V);

const value = choose<number>(1);
value[0] satisfies number;
value[1] satisfies number;
value[2] satisfies number;
```
