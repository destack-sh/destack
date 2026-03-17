# Widening Commitment Boundaries

## literal retention

### const assertions keep nested literals through satisfies

> `as const` values checked with `satisfies` should keep nested literal precision instead of widening.

```ts
const value = ({ env: { mode: "dev" } } as const) satisfies { env: { mode: string } };

value.env.mode satisfies "dev";
```

### generic const wrappers preserve literal precision

> Generic wrappers over readonly inputs should preserve literal precision through commitment boundaries.

```ts
declare function freeze<const T>(value: T): T;

const value = freeze({ kind: "ready", level: 1 });
value.kind satisfies "ready";
value.level satisfies 1;
```

## widening through mutability

### mutable generic wrappers widen object literal members

> Passing literals through mutable generic wrappers should commit members to widened mutable types.

```ts
declare function hold<T>(value: T): T;

let value = hold({ kind: "ready" });
value.kind satisfies "ready";
```

- contains: not assignable

### mutable spread targets widen readonly source literals

> Spreading readonly sources into mutable object targets should widen member literals at the target commitment site.

```ts
const base = { kind: "ready" } as const;
let value = { ...base };

value.kind satisfies "ready";
```

- contains: not assignable

## freshness and excess

### direct fresh literals reject excess fields at commitment

> A fresh literal committed directly against a target type should still trigger excess property rejection.

```ts
type Ready = { kind: "ready"; payload: string };

declare function accept(value: Ready): void;

accept({ kind: "ready", payload: "ok", extra: true });
```

- contains: excess property

### stale values remain assignable after variable commitment

> The same shape, once stale through variable commitment, should remain assignable despite extra properties.

```ts
type Ready = { kind: "ready"; payload: string };

declare function accept(value: Ready): void;

const value = { kind: "ready" as const, payload: "ok", extra: true };
accept(value);
```

```json:destack.json
{ "compilerOptions": { "allowTs": true, "checkTs": true } }
```
