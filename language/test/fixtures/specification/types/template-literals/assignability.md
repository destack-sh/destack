# Template Literal Assignability

## template literal type assigns to broader template

> Narrow template literal types assign to broader ones.

```ds
type Loose = `${string}-id`;
type Tight = `user-${string}-id`;

declare let tight: Tight;
let ok: Loose = tight;
```

## template literal type rejects assignment to narrower template

> Broader template literal types do not assign to narrower ones.

```ds
type Loose = `${string}-id`;
type Tight = `user-${string}-id`;

declare let loose: Loose;
let bad: Tight = loose;
```

- contains: type `${string}-id` is not assignable to type `user-${string}-id`

## template literal type accepts generic spans

> Generic spans assign to the string supertype.

```ds
type AnyString = `${string}`;
type Tagged = `tag-${string}`;

declare let tagged: Tagged;
let ok: AnyString = tagged;
```

## template literal type assigns to string

> Template literal types assign to string.

```ds
type Tagged = `tag-${string}`;

declare let tagged: Tagged;
let ok: string = tagged;
```

## string does not assign to template literal type

> Strings do not assign to narrower template literal types.

```ds
type Tagged = `tag-${string}`;

declare let value: string;
let bad: Tagged = value;
```

- contains: type string is not assignable to type `tag-${string}`
