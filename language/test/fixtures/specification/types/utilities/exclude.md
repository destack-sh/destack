# Exclude

`Exclude` is a standard utility type.

## unions

### exclude distributes over unions

Matching arms drop.

```ds
type Letters = "a" | "b" | "c";
type Only = Exclude<Letters, "b">;

const ok: Only = "a";
ok satisfies Only;
```

### exclude rejects removed members

Excluded arms are gone.

```ds
type Letters = "a" | "b" | "c";
type Only = Exclude<Letters, "b">;

const bad: Only = "b";
```

- contains: not assignable

### exclude with never yields never

Nothing excludes to nothing.

```ds
type NeverLetters = Exclude<never, "b">;

let bad: NeverLetters = "b";
```

- contains: not assignable
