# Derive

Derive providers add ordinary declarations to nominal types.

## nominal types

### derive applies to structs

Derive decorators apply to nominal type declarations.

```ds
@derive(Clone)
struct User {
    id: int64;
}

const user = User { id: 1 };
user.clone() satisfies User;
```

### derive can request multiple providers

Each derive argument names one provider.

```ds
@derive(Clone, Debug)
struct User {
    id: int64;
}

const user = User { id: 1 };
user.clone() satisfies User;
user.debug() satisfies string;
```

### module derives run automatically

Auto derives apply to every eligible declaration in the module.

```ds
module {
    const derive = ["Clone"];
}

struct User {
    id: int64;
}

const user = User { id: 1 };
user.clone() satisfies User;
```
