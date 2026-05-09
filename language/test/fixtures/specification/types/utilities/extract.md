# Extract

`Extract` is a standard utility type.

## unions

### extract keeps matching members

```ds
type Letters = "a" | "b" | "c";
type OnlyAorB = Extract<Letters, "a" | "b">;

const ok: OnlyAorB = "a";
ok satisfies OnlyAorB;
const ok2: OnlyAorB = "b";
ok2 satisfies OnlyAorB;
```

### extract rejects non members

```ds
type Letters = "a" | "b" | "c";
type OnlyAorB = Extract<Letters, "a" | "b">;

const bad: OnlyAorB = "c";
```

- contains: not assignable

### extract with never yields never

```ds
type NeverLetters = Extract<never, "a">;

let bad: NeverLetters = "a";
```

- contains: not assignable
