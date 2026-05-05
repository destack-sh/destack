# Exclude

`Exclude` is a standard utility type.

### exclude distributes over unions

```ds
type Letters = "a" | "b" | "c";
type Only = Exclude<Letters, "b">;

const ok: Only = "a";
ok satisfies Only;
```

### exclude rejects removed members

```ds
type Letters = "a" | "b" | "c";
type Only = Exclude<Letters, "b">;

const bad: Only = "b";
```

- contains: not assignable

### exclude with never yields never

```ds
type NeverLetters = Exclude<never, "b">;

let bad: NeverLetters = "b";
```

- contains: not assignable

