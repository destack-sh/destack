# Struct Decorators

Tests for decorators on struct declarations and members.

## struct decorators

### struct member decorators are accepted

> Decorators can be applied to struct fields.

```ds
struct User {
    @deprecated
    name: string;
}

const user = User { name: "Ada" };
user.name satisfies string;
```

### struct declaration decorators are accepted

> Decorators can be applied to struct declarations.

```ds
@hot
struct User {
    name: string;
}

const user = User { name: "Ada" };
user.name satisfies string;
```

### struct decorators reject conflicting likely and unlikely

> likely and unlikely decorators cannot be combined on struct declarations.

```ds
@likely
@unlikely
struct User {
    name: string;
}
```

- contains: likely and unlikely decorators cannot be combined

### struct field decorators reject conflicting likely and unlikely

> likely and unlikely decorators cannot be combined on struct fields.

```ds
struct User {
    @likely
    @unlikely
    name: string;
}
```

- contains: likely and unlikely decorators cannot be combined

### struct field decorators can stack non-conflicting attributes

> Non-conflicting decorators can stack on struct fields.

```ds
struct User {
    @deprecated
    @cold
    name: string;
}

const user = User { name: "Ada" };
user.name satisfies string;
```
