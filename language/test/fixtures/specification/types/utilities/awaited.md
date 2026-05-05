# Awaited

`Awaited` unwraps the value produced by `await`.

### awaited keeps non promises

```ds
type Value = Awaited<string>;

const ok: Value = "ready";
ok satisfies string;
```

### awaited unwraps promise-like values

```ds
type Value = Awaited<PromiseLike<string>>;

const ok: Value = "ready";
ok satisfies string;
```

### awaited unwraps nested promise-like values

```ds
type Value = Awaited<PromiseLike<PromiseLike<string>>>;

const ok: Value = "ready";
ok satisfies string;
```

### awaited rejects unresolved promise values

```ds
type Value = Awaited<PromiseLike<string>>;

const bad: Value = Promise.resolve("ready");
```

- contains: not assignable

### awaited preserves nullish values

```ds
type Value = Awaited<null | undefined>;

const ok: Value = null;
const ok2: Value = undefined;
```
