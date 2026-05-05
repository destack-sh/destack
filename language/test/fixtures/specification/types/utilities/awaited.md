# Awaited

`Awaited` unwraps the value produced by `await`.

## cases

### awaited keeps non promises

> Non thenable values are preserved.

```ts libs=es5
type Value = Awaited<string>;

const ok: Value = "ready";
ok satisfies string;
```

### awaited unwraps promise-like values

> Promise-like values unwrap to their fulfilled value.

```ts libs=es5
type Value = Awaited<PromiseLike<string>>;

const ok: Value = "ready";
ok satisfies string;
```

### awaited unwraps nested promise-like values

> Nested promise-like values unwrap recursively.

```ts libs=es5
type Value = Awaited<PromiseLike<PromiseLike<string>>>;

const ok: Value = "ready";
ok satisfies string;
```

### awaited rejects unresolved promise values

> The original promise-like value is not assignable after unwrapping.

```ts libs=es5
type Value = Awaited<PromiseLike<string>>;

const bad: Value = Promise.resolve("ready");
```

- contains: not assignable

### awaited preserves nullish values

> Nullish values are preserved.

```ts libs=es5
type Value = Awaited<null | undefined>;

const ok: Value = null;
const ok2: Value = undefined;
```
