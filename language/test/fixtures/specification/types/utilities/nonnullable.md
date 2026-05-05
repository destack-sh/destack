# NonNullable

`NonNullable` is a standard utility type.

### NonNullable removes nullish

```ds
type MaybeName = string | null | undefined;
type Name = NonNullable<MaybeName>;

const ok: Name = "Ada";
ok satisfies Name;
```

### NonNullable rejects nullish values

```ds
type MaybeName = string | null | undefined;
type Name = NonNullable<MaybeName>;

const bad: Name = null;
```

- contains: not assignable

### NonNullable with never yields never

```ds
type NeverValue = NonNullable<never>;

let bad: NeverValue = "no";
```

- contains: not assignable
