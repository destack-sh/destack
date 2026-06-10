# NonNullable

`NonNullable` is a standard utility type.

## unions

### NonNullable removes nullish

Both nullish arms drop.

```ds
type MaybeName = string | null | undefined;
type Name = NonNullable<MaybeName>;

const ok: Name = "Ada";
ok satisfies Name;
```

### NonNullable rejects nullish values

The nullish arms are gone.

```ds
type MaybeName = string | null | undefined;
type Name = NonNullable<MaybeName>;

const bad: Name = null;
```

- contains: not assignable

### NonNullable with never yields never

Nothing minus anything is nothing.

```ds
type NeverValue = NonNullable<never>;

let bad: NeverValue = "no";
```

- contains: not assignable
