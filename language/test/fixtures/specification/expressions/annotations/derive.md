# Derive

`@derive(...)` asks the compiler to add ordinary declarations for a nominal type.

## nominal types

### derive applies to structs

Derive annotations apply to nominal type declarations.

```ds
@derive(Clone)
struct User {
    id: int64;
}

const user = User { id: 1 };
user.clone() satisfies User;
```

### derive can request multiple capabilities

Each derive argument names one nominal capability.

```ds
@derive(Clone, Debug)
struct User {
    id: int64;
}

const user = User { id: 1 };
user.clone() satisfies User;
user.debug() satisfies string;
```

### derive rejects structural aliases

Derive output is declaration-shaped and needs a nominal target.

```ds
@derive(Clone)
type User = { id: int64 };
```

- contains: nominal

### derive rejects unresolved capabilities

Derive arguments must resolve.

```ds
@derive(Missing)
struct User {
    id: int64;
}
```

- contains: Missing
