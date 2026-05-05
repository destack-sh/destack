# Exclude

`Exclude` is a standard TypeScript utility type.

## cases

### exclude distributes over unions

> Conditional types distribute over union types.

```ds libs=es5
type Letters = "a" | "b" | "c";
type Only = Exclude<Letters, "b">;

const ok: Only = "a";
ok satisfies Only;
```

### exclude rejects removed members

> Excluded members are not assignable.

```ds libs=es5
type Letters = "a" | "b" | "c";
type Only = Exclude<Letters, "b">;

const bad: Only = "b";
```

- contains: not assignable

### exclude with never yields never

> Excluding from never yields never.

```ds libs=es5
type NeverLetters = Exclude<never, "b">;

let bad: NeverLetters = "b";
```

- contains: not assignable

