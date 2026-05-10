# Freshness

## freshness boundaries

### direct object literals preserve discriminant freshness for excess checks

A direct object-literal argument stays fresh so excess property checking still runs at the call boundary.

```ds
type Ready = { kind: "ready"; payload: string };

declare const use_ready: (value: Ready) => void;

use_ready({ kind: "ready", payload: "ok", extra: true });
```

- contains: excess property

### variable indirection drops freshness for excess checks

Once the same literal flows through a variable binding, freshness is dropped and excess checks relax.

```ds
type Ready = { kind: "ready"; payload: string };

declare const use_ready: (value: Ready) => void;

const input = { kind: "ready" as const, payload: "ok", extra: true };
use_ready(input);
```

### const assertions preserve nested literal members through wrappers

`as const` wrappers preserve nested literal member precision across generic wrapper calls.

```ds
declare function pass<T>(value: T): T;

const config = pass({ env: { mode: "dev" } } as const);
config.env.mode satisfies "dev";
```

### mutable wrappers widen nested literal members

Mutable generic wrappers widen nested literals to mutable member types.

```ds
declare function pass<T>(value: T): T;

let config = pass({ env: { mode: "dev" } });
config.env.mode satisfies "dev";
```

- contains: not assignable
