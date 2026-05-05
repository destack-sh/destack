# Parameters

`Parameters` extracts a function parameter tuple.

## cases

### parameters extracts argument types

> Function parameters are returned as a tuple.

```ts libs=es5
type Args = Parameters<(name: string, count: number) => boolean>;

const ok: Args = ["Ada", 1];
ok satisfies [string, number];
```

### parameters rejects wrong argument types

> Extracted parameter tuples keep each parameter type.

```ts libs=es5
type Args = Parameters<(name: string, count: number) => boolean>;

const bad: Args = ["Ada", "one"];
```

- contains: not assignable

### parameters preserves optional parameters

> Optional parameters stay optional in the tuple.

```ts libs=es5
type Args = Parameters<(name: string, count?: number) => boolean>;

const ok: Args = ["Ada"];
const ok2: Args = ["Ada", 1];
```

### parameters preserves rest parameters

> Rest parameters stay rest-like in the tuple.

```ts libs=es5
type Args = Parameters<(name: string, ...flags: boolean[]) => void>;

const ok: Args = ["Ada", true, false];
```
