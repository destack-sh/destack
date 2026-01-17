# Type Descriptors

Tests for `Type<T>` descriptors and `typeOf`.

## type as value

### type as value exposes descriptor members

> Using a type name in value position exposes descriptor members.

```ds
struct User { name: string }

const UserType = User;
UserType.name satisfies string;
```

### type descriptor exposes name

> Type descriptors expose the name of the underlying type.

```ds
struct User { name: string }

const name = User.name;
name satisfies string;
```

## typeOf

### typeOf returns Type<T>

> typeOf returns a `Type<T>` descriptor for a value.

```ds
struct User { name: string }

const value = User { name: "Ada" };
const descriptor = typeOf(value);
descriptor satisfies Type<User>;
```

### type guard returns boolean

> Type descriptors provide runtime guard predicates via `is`.

```ds
struct User { name: string }

const value: unknown = User { name: "Ada" };
const ok = User.is(value);
ok satisfies boolean;
```
