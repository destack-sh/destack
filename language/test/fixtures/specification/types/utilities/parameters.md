# Parameters

`Parameters` extracts a function parameter tuple.

## functions

### parameters extracts argument types

The parameter list becomes a tuple.

```ds
type Args = Parameters<(name: string, count: number) => boolean>;

const ok: Args = ("Ada", 1);
ok satisfies (string, number);
```

### parameters rejects wrong argument types

The tuple is exact.

```ds
type Args = Parameters<(name: string, count: number) => boolean>;

const bad: Args = ("Ada", "one");
```

- contains: not assignable

### parameters preserves optional parameters

Optional parameters become optional elements.

```ds
type Args = Parameters<(name: string, count?: number) => boolean>;

const ok: Args = ("Ada",);
const ok2: Args = ("Ada", 1);
```

### parameters preserves rest parameters

Rest parameters become rest elements.

```ds
type Args = Parameters<(name: string, ...flags: boolean[]) => void>;

const ok: Args = ("Ada", true, false);
```
