# Excess Property Checks

## unions

### union rejects extra fields

> Object literals with fields not present in any union member are rejected.

```ds
interface Named {
    name: string
}

interface Aged {
    age: int32
}

const value: Named | Aged = { name: "Ada", extra: true };
```

- contains: excess property

## intersections

### intersection requires all fields

> Object literals must satisfy every intersection member.

```ds
interface Named {
    name: string
}

interface Aged {
    age: int32
}

const value: Named & Aged = { name: "Ada" };
```

- contains: not assignable

### intersection accepts combined fields

> Object literals satisfy intersections when all fields are present.

```ds
interface Named {
    name: string
}

interface Aged {
    age: int32
}

const value: Named & Aged = { name: "Ada", age: 42 };
```

## freshness boundaries

### fresh object literals reject extra fields for direct object targets

> Fresh object literals enforce excess property checks for direct object targets.

```ts
type Person = { name: string };

const value: Person = { name: "Ada", extra: true };
```

- contains: excess property

### non-fresh object values allow extra fields through assignment

> Non-fresh object values do not run excess property checks when assigned later.

```ts
type Person = { name: string };

const source = { name: "Ada", extra: true };
const value: Person = source;
```

### generic targets do not run excess checks for fresh literals

> Generic target positions infer full source shape without excess checks.

```ts
function keep<T extends { name: string }>(value: T): T {
    return value;
}

const value = keep({ name: "Ada", extra: true });
value.extra satisfies boolean;
```

## discriminated unions

### fresh discriminant literals reject extra fields

> Fresh literals targeting discriminated unions reject non-member fields.

```ts
type Shape =
    | { kind: "a"; value: number }
    | { kind: "b"; value: string };

const value: Shape = { kind: "a" as const, value: 1, extra: true };
```

- contains: not assignable

### non-fresh discriminant values allow extra fields

> Non-fresh values may carry extra fields when structurally assignable to a member.

```ts
type Shape =
    | { kind: "a"; value: number }
    | { kind: "b"; value: string };

const source = { kind: "a" as const, value: 1, extra: true };
const value: Shape = source;
```

## spread freshness

### fresh spread literals reject explicit extra fields

> Fresh spread literals still reject explicit excess fields on the literal itself.

```ts
type Person = { name: string };

const base = { name: "Ada" };
const value: Person = { ...base, extra: true };
```

- contains: excess property

### spread-only object literals keep source extras without fresh excess checks

> Spread-only object literals do not re-run fresh excess checks on spread-origin fields.

```ts
type Person = { name: string };

const source = { name: "Ada", extra: true };
const value: Person = { ...source };
```

## weak type overlap

### weak object targets reject assignments without shared properties

> Weak object target types reject sources that do not share required keys.

```ts
type WeakPoint = {
    x?: number;
    y?: number;
};

const value: WeakPoint = { label: "origin" };
```

- contains: has no properties in common

### weak object targets accept assignments with shared properties

> Weak object target types accept sources that share at least one target property.

```ts
type WeakPoint = {
    x?: number;
    y?: number;
};

const value: WeakPoint = { x: 1, label: "origin" };
```

### weak object targets reject fresh spread literals without shared properties

> Fresh spread literals still reject weak targets when no target property is shared.

```ts
type WeakPoint = {
    x?: number;
    y?: number;
};

const base = { label: "origin" };
const value: WeakPoint = { ...base };
```

- contains: has no properties in common

### weak object targets accept spread literals with shared properties

> Spread literals are accepted for weak targets when at least one target property is shared.

```ts
type WeakPoint = {
    x?: number;
    y?: number;
};

const base = { x: 1, label: "origin" };
const value: WeakPoint = { ...base };
```

### callback return literals run excess checks in contextual object return types

> Contextually typed callback returns run excess property checks on fresh returned literals.

```ts
type Person = { name: string };

function use(factory: () => Person): Person {
    return factory();
}

use(() => ({ name: "Ada", extra: true }));
```

- contains: excess property

### callback-produced values lose freshness outside contextual return positions

> Object values produced by non-contextual callbacks do not run excess checks on later assignment.

```ts
type Person = { name: string };

const make = () => ({ name: "Ada", extra: true });
const value: Person = make();
```

### fresh discriminant literals reject extra fields through renamed re-exports

> Fresh discriminant literals reject excess fields through renamed re-export paths.

```ts:shape.ts
export type Shape =
    | { kind: "a"; value: number }
    | { kind: "b"; value: string };
```

```ts:index.ts
export type { Shape as PublicShape } from "./shape";
```

```ts:main.ts
import type { PublicShape } from "./index";

const value: PublicShape = { kind: "a" as const, value: 1, extra: true };
```

- contains: excess property

### spread discriminant literals reject extra fields through renamed re-exports

> Fresh spread literals still reject explicit excess fields for discriminant union targets through renamed re-exports.

```ts:shape.ts
export type Shape =
    | { kind: "a"; value: number }
    | { kind: "b"; value: string };
```

```ts:index.ts
export type { Shape as PublicShape } from "./shape";
```

```ts:main.ts
import type { PublicShape } from "./index";

const base = { kind: "a" as const, value: 1 };
const value: PublicShape = { ...base, extra: true };
```

- contains: excess property
