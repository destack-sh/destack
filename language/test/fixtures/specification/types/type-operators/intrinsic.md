# Intrinsic Types

## intrinsic indexing

### intrinsic types cannot be indexed

```ts:main.ts
type Bad = intrinsic["foo"];
```

- contains: intrinsic types cannot be indexed

### intrinsic types cannot appear in value annotations

```ts:main.ts
const value: intrinsic = "x";
```

- contains: intrinsic

### intrinsic types cannot be used in conditional operators

```ts:main.ts
type Select<T> = intrinsic extends T ? true : false;
```

- contains: intrinsic

### intrinsic types cannot be passed as generic arguments

```ts:main.ts
type Box<T> = T;
type Bad = Box<intrinsic>;
```

- contains: intrinsic

### intrinsic types cannot appear in union members

```ts:main.ts
type Bad = intrinsic | string;
```

- contains: intrinsic
