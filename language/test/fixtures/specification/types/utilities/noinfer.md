# NoInfer

`NoInfer` blocks inference from a selected position.

## cases

### noinfer keeps inference from earlier arguments

> A later `NoInfer` argument must match the type inferred from earlier arguments.

```ts libs=es5
declare function choose<C extends string>(values: C[], fallback?: NoInfer<C>): C;

const ok = choose(["red", "blue"], "red");
ok satisfies "red" | "blue";
```

### noinfer rejects unrelated later arguments

> The `NoInfer` position cannot add new candidates.

```ts libs=es5
declare function choose<C extends string>(values: C[], fallback?: NoInfer<C>): C;

choose(["red", "blue"], "green");
```

- contains: not assignable
