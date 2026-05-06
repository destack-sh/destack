# Bindings

## const bindings

### const assertions keep nested literals through satisfies

`as const` values checked with `satisfies` keep nested literal precision.

```ds
const value = ({ env: { mode: "dev" } } as const) satisfies { env: { mode: string } };

value.env.mode satisfies "dev";
```

### generic const wrappers preserve literal precision

Generic wrappers over readonly inputs preserve literal precision when the value is assigned.

```ds
declare function freeze<const T>(value: T): T;

const value = freeze({ kind: "ready", level: 1 });
value.kind satisfies "ready";
value.level satisfies 1;
```

## mutable bindings

### mutable generic wrappers widen object literal members

Passing literals through mutable generic wrappers widens members to mutable types.

```ds
declare function hold<T>(value: T): T;

let value = hold({ kind: "ready" });
value.kind satisfies "ready";
```

- contains: not assignable

### mutable spread targets widen readonly source literals

Spreading readonly sources into mutable object targets widens member literals at the assignment site.

```ds
const base = { kind: "ready" } as const;
let value = { ...base };

value.kind satisfies "ready";
```

- contains: not assignable

## object freshness

### direct fresh literals reject excess fields at typed calls

A fresh literal passed directly to a target type still triggers excess property rejection.

```ds
type Ready = { kind: "ready"; payload: string };

declare function accept(value: Ready): void;

accept({ kind: "ready", payload: "ok", extra: true });
```

- contains: excess property

### stale values remain assignable after variable binding

The same shape, once held in a variable, remains assignable despite extra properties.

```ds
type Ready = { kind: "ready"; payload: string };

declare function accept(value: Ready): void;

const value = { kind: "ready" as const, payload: "ok", extra: true };
accept(value);
```
