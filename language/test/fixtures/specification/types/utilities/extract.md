# Extract

`Extract` is a standard TypeScript utility type.

## cases

### extract keeps matching members

> `Extract` keeps the overlapping members.

```ds libs=es5
type Letters = "a" | "b" | "c";
type OnlyAorB = Extract<Letters, "a" | "b">;

const ok: OnlyAorB = "a";
ok satisfies OnlyAorB;
const ok2: OnlyAorB = "b";
ok2 satisfies OnlyAorB;
```

### extract rejects non members

> Non matching members are rejected.

```ds libs=es5
type Letters = "a" | "b" | "c";
type OnlyAorB = Extract<Letters, "a" | "b">;

const bad: OnlyAorB = "c";
```

- contains: not assignable

### extract with never yields never

> Extracting from never yields never.

```ds libs=es5
type NeverLetters = Extract<never, "a">;

let bad: NeverLetters = "a";
```

- contains: not assignable
