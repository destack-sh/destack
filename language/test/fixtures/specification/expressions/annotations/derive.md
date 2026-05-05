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
