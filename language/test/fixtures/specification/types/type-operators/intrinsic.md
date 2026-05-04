# Intrinsic Types

## intrinsic indexing

### intrinsic types cannot be indexed

> Intrinsic types cannot be indexed.

```ts:main.ts
type Bad = intrinsic["foo"];
```

- intrinsic types cannot be indexed

### intrinsic types cannot appear in value annotations

> Intrinsic marker types cannot be used as concrete value annotations.

```ts:main.ts
const value: intrinsic = "x";
```

- contains: intrinsic

### intrinsic types cannot be used in conditional operators

> Intrinsic marker types cannot participate in user-defined conditional types.

```ts:main.ts
type Select<T> = intrinsic extends T ? true : false;
```

- contains: intrinsic

### intrinsic types cannot be passed as generic arguments

> Intrinsic marker types cannot be supplied as user-defined generic arguments.

```ts:main.ts
type Box<T> = T;
type Bad = Box<intrinsic>;
```

- contains: intrinsic

### intrinsic types cannot appear in union members

> Intrinsic marker types cannot participate in user-defined union types.

```ts:main.ts
type Bad = intrinsic | string;
```

- contains: intrinsic
