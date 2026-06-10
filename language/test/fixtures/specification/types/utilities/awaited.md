# Awaited

`Awaited` unwraps the value produced by `await`.

## unwrapping

### awaited keeps non promises

Plain values pass through.

```ds
type Value = Awaited<string>;

const ok: Value = "ready";
ok satisfies string;
```

### awaited unwraps promise-like values

One promise layer unwraps.

```ds
type Value = Awaited<Promise<string>>;

const ok: Value = "ready";
ok satisfies string;
```

### awaited unwraps nested promise-like values

Unwrapping recurses.

```ds
type Value = Awaited<Promise<Promise<string>>>;

const ok: Value = "ready";
ok satisfies string;
```

### awaited rejects unresolved promise values

The unwrapped type is not the promise.

```ds
type Value = Awaited<Promise<string>>;

const bad: Value = Promise.resolve("ready");
```

- contains: not assignable

### awaited preserves nullish values

Nullish values are not promises.

```ds
type Value = Awaited<null | undefined>;

const ok: Value = null;
const ok2: Value = undefined;
```
