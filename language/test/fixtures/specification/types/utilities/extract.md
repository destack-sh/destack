# Extract

`Extract` is a standard TypeScript utility type.

### extract keeps matching members

```ds libs=es5
type Letters = "a" | "b" | "c";
type OnlyAorB = Extract<Letters, "a" | "b">;

const ok: OnlyAorB = "a";
ok satisfies OnlyAorB;
const ok2: OnlyAorB = "b";
ok2 satisfies OnlyAorB;
```

### extract rejects non members

```ds libs=es5
type Letters = "a" | "b" | "c";
type OnlyAorB = Extract<Letters, "a" | "b">;

const bad: OnlyAorB = "c";
```

- contains: not assignable

### extract with never yields never

```ds libs=es5
type NeverLetters = Extract<never, "a">;

let bad: NeverLetters = "a";
```

- contains: not assignable
