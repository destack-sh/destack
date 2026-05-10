# Mapped Types

## Mapped Types

### mapped type keeps bracket spacing

Mapped types include spaces when bracket spacing is enabled.

```ts:main.ts
export type Bar<T> = {[P in keyof T]: string}
```

```ts expected
export type Bar<T> = { [P in keyof T]: string };
```

### mapped type modifiers

Mapped type modifiers keep their prefixes and suffixes.

```ts:main.ts
type ReadonlyPartial<T> = { readonly [K in keyof T]?: T[K] }
type Mutable<T> = { -readonly [K in keyof T]-?: T[K] }
```

```ts expected
type ReadonlyPartial<T> = { readonly [K in keyof T]?: T[K] };
type Mutable<T> = { -readonly [K in keyof T]-?: T[K] };
```

### mapped type key remap

Mapped type key remaps keep `as` spacing.

```ts:main.ts
type EventHandlers<T> = { [K in keyof T as `on${Capitalize<K & string>}`]?: T[K] }
```

```ts expected
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

```ts:main.ts indent-width=2 line-width=80
export type Bar<T> = {[P in keyof T]: string}
```

```ts expected
export type Bar<T> = { [P in keyof T]: string };
```

### mapped type key remapping keeps remap expressions

Mapped types with `as` remaps preserve their remap expressions under non-default formatter options.

```ts:main.ts indent-width=2 line-width=80
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

```ts expected
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
