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
