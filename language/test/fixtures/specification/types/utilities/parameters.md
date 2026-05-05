# Parameters

`Parameters` extracts a function parameter tuple.

### parameters extracts argument types

```ds
type Args = Parameters<(name: string, count: number) => boolean>;

const ok: Args = ("Ada", 1);
ok satisfies (string, number);
```

### parameters rejects wrong argument types

```ds
type Args = Parameters<(name: string, count: number) => boolean>;

const bad: Args = ("Ada", "one");
```

- contains: not assignable

### parameters preserves optional parameters

```ds
type Args = Parameters<(name: string, count?: number) => boolean>;

const ok: Args = ("Ada",);
const ok2: Args = ("Ada", 1);
```

### parameters preserves rest parameters

```ds
type Args = Parameters<(name: string, ...flags: boolean[]) => void>;

const ok: Args = ("Ada", true, false);
```
