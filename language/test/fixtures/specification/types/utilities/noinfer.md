# NoInfer

`NoInfer` blocks inference from a selected position.

## inference

### NoInfer keeps inference from earlier arguments

```ds
declare function choose<C extends string>(values: C[], fallback?: NoInfer<C>): C;

const ok = choose(["red", "blue"], "red");
ok satisfies "red" | "blue";
```

### NoInfer rejects unrelated later arguments

```ds
declare function choose<C extends string>(values: C[], fallback?: NoInfer<C>): C;

choose(["red", "blue"], "green");
```

- contains: not assignable
