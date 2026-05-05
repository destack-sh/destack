# As

`as` asserts the expression type to the target type when the source and target overlap.

## assertions

### as preserves the asserted type

> `as` applies the target type.

```ds
const value = "ok" as string;
value satisfies string;
```

### as can narrow unions

> `as` can select a narrower union member.

```ds
const value = ("ok" as string | number) as string;
value satisfies string;
```

### as can widen to supertypes

> A subtype can be asserted to a supertype.

```ds
const value: 42 = 42;
const widened = value as number;
widened satisfies number;
```

### as can narrow from unknown

> `unknown` can be asserted to a concrete type.

```ds
declare const value: unknown;

const narrowed = value as number;
narrowed satisfies number;
```

### as can widen object shapes

> An object can be asserted to a type with fewer required fields.

```ds
const value = { x: 1, y: 2 };
const narrowed = value as { x: number };
narrowed satisfies { x: number };
```

## rejections

### as rejects unrelated types

> `as` rejects unrelated source and target types.

```ds
const value: string = "hello";
const numberValue = value as number;
```

- contains: cannot cast type string to number

### as rejects boolean to number

> Boolean and number do not overlap.

```ds
const value: boolean = true;
const numberValue = value as number;
```

- contains: cannot cast type boolean to number
