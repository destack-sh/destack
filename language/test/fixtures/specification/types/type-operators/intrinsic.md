# Intrinsic Types

## intrinsic indexing

### intrinsic types cannot be indexed

```ds:main.ds
type Bad = intrinsic["foo"];
```

- contains: intrinsic types cannot be indexed

### intrinsic types cannot appear in value annotations

```ds:main.ds
const value: intrinsic = "x";
```

- contains: intrinsic

### intrinsic types cannot be used in conditional operators

```ds:main.ds
type Select<T> = intrinsic extends T ? true : false;
```

- contains: intrinsic

### intrinsic types cannot be passed as generic arguments

```ds:main.ds
type Box<T> = T;
type Bad = Box<intrinsic>;
```

- contains: intrinsic

### intrinsic types cannot appear in union members

```ds:main.ds
type Bad = intrinsic | string;
```

- contains: intrinsic
