# NonNullable

`NonNullable` is a standard TypeScript utility type.

### NonNullable removes nullish

```ds libs=es5
type MaybeName = string | null | undefined;
type Name = NonNullable<MaybeName>;

const ok: Name = "Ada";
ok satisfies Name;
```

### NonNullable rejects nullish values

```ds libs=es5
type MaybeName = string | null | undefined;
type Name = NonNullable<MaybeName>;

const bad: Name = null;
```

- contains: not assignable

### NonNullable with never yields never

```ds libs=es5
type NeverValue = NonNullable<never>;

let bad: NeverValue = "no";
```

- contains: not assignable
