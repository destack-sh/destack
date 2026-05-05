# Exclude

`Exclude` is a standard TypeScript utility type.

### exclude distributes over unions

```ds libs=es5
type Letters = "a" | "b" | "c";
type Only = Exclude<Letters, "b">;

const ok: Only = "a";
ok satisfies Only;
```

### exclude rejects removed members

```ds libs=es5
type Letters = "a" | "b" | "c";
type Only = Exclude<Letters, "b">;

const bad: Only = "b";
```

- contains: not assignable

### exclude with never yields never

```ds libs=es5
type NeverLetters = Exclude<never, "b">;

let bad: NeverLetters = "b";
```

- contains: not assignable

