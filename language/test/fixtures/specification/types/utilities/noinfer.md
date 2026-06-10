# NoInfer

`NoInfer` blocks inference from a selected position.

## inference

### NoInfer keeps inference from earlier arguments

The wrapped position does not vote on inference.

```ds
declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C;

const ok = choose(["red", "blue"], "red");
ok satisfies "red" | "blue";
```

### NoInfer rejects unrelated later arguments

The inferred type still checks the wrapped argument.

```ds
declare function choose<C: string>(values: C[], fallback?: NoInfer<C>): C;

choose(["red", "blue"], "green");
```

- contains: not assignable
