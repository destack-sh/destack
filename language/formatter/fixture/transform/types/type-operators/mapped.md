# Mapped Types

## Mapped Types

### mapped type keeps bracket spacing

Mapped types include spaces when bracket spacing is enabled.

```ds:main.ds
export type Bar<T> = {[P in keyof T]: string}
```

```ds expected
export type Bar<T> = { [P in keyof T]: string };
```

### mapped type modifiers

Mapped type modifiers keep their prefixes and suffixes.

```ds:main.ds
type ReadonlyPartial<T> = { readonly [K in keyof T]?: T[K] }
type Mutable<T> = { -readonly [K in keyof T]-?: T[K] }
```

```ds expected
type ReadonlyPartial<T> = { readonly [K in keyof T]?: T[K] };
type Mutable<T> = { -readonly [K in keyof T]-?: T[K] };
```

### mapped type key remap

Mapped type key remaps keep `as` spacing.

```ds:main.ds
type EventHandlers<T> = { [K in keyof T as `on${Capitalize<K & string>}`]?: T[K] }
```

```ds expected
type EventHandlers<T> = { [K in keyof T as `on${Capitalize<K & string>}`]?: T[K] };
```

### mapped type separator comments

Mapped type comments format at remap and value boundaries.

```ds
type Flags<T> = {
  [K in keyof T as /* remap */ `can${Capitalize<K & string>}`]: /* value */ boolean
}

type Values<T> = {
  [K in keyof T]: // value-line
    boolean
}
```

```ds expected
type Flags<T> = {
    [K in keyof T as /* remap */ `can${Capitalize<K & string>}`]: /* value */ boolean;
};

type Values<T> = {
    [K in keyof T]: boolean; // value-line
};
```

### exported mapped type stays inline

Short exported mapped types stay inline under non-default formatter options.

```ds:main.ds indent-width=2 line-width=80
export type Bar<T> = {[P in keyof T]: string}
```

```ds expected
export type Bar<T> = { [P in keyof T]: string };
```

### mapped type key remapping keeps remap expressions

Mapped types with `as` remaps preserve their remap expressions under non-default formatter options.

```ds:main.ds indent-width=2 line-width=80
type MappedTypeWithNewKeys<T> = {
  [K in keyof T as NewKeyType]: T[K]
};

type RemoveKindField<T> = {
  [K in keyof T as Exclude<K, "kind">]: T[K]
};

type PickByValueType<T, U> = {
  [K in keyof T as T[K] extends U ? K : never]: T[K]
};
```

```ds expected
type MappedTypeWithNewKeys<T> = {
  [K in keyof T as NewKeyType]: T[K];
};

type RemoveKindField<T> = {
  [K in keyof T as Exclude<K, "kind">]: T[K];
};

type PickByValueType<T, U> = {
  [K in keyof T as T[K] extends U ? K : never]: T[K];
};
```
