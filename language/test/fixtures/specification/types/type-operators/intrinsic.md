# Intrinsic Types

`intrinsic` marks compiler-provided types and is otherwise inert.

## intrinsic indexing

### intrinsic types cannot be indexed

`intrinsic` is a marker, not a type.

```ds:main.ds
type Bad = intrinsic["foo"];
```

- contains: intrinsic types cannot be indexed

### intrinsic types cannot appear in value annotations

Values cannot have it.

```ds:main.ds
const value: intrinsic = "x";
```

- contains: intrinsic

### intrinsic types cannot be used in conditional operators

Type algebra cannot see it.

```ds:main.ds
type Select<T> = intrinsic extends T ? true : false;
```

- contains: intrinsic

### intrinsic types cannot be passed as generic arguments

It does not instantiate.

```ds:main.ds
type Box<T> = T;
type Bad = Box<intrinsic>;
```

- contains: intrinsic

### intrinsic types cannot appear in union members

It does not combine.

```ds:main.ds
type Bad = intrinsic | string;
```

- contains: intrinsic
