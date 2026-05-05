# NonNullable

`NonNullable` is a standard TypeScript utility type.

## cases

### nonnullable removes nullish

> `NonNullable` strips nullish members.

```ds libs=es5
type MaybeName = string | null | undefined;
type Name = NonNullable<MaybeName>;

const ok: Name = "Ada";
ok satisfies Name;
```

### nonnullable rejects nullish values

> Nullish values are rejected.

```ds libs=es5
type MaybeName = string | null | undefined;
type Name = NonNullable<MaybeName>;

const bad: Name = null;
```

- contains: not assignable

### nonnullable with never yields never

> NonNullable preserves never.

```ds libs=es5
type NeverValue = NonNullable<never>;

let bad: NeverValue = "no";
```

- contains: not assignable

