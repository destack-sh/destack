# Type Descriptors

Tests for `Type<T>` descriptors and `typeOf`.

## type as value

FUGU #Incomplete: type descriptors / RTTI inference (with inference)

### type as value yields descriptor

> Using a type name in value position yields a `Type<T>` descriptor.

```ds
struct User { name: string }

const descriptor = User;
descriptor satisfies Type<User>;
```

### type descriptor exposes base members

> Type descriptors expose base members like `name`, `id`, and `is`.

```ds
struct User { name: string }

const descriptor = User;
const name = descriptor.name;
const id = descriptor.id;
const ok = descriptor.is(User { name: "Ada" });

name satisfies string;
id satisfies TypeId;
ok satisfies boolean;
```

### type descriptor narrows by kind

> Descriptor variants can be narrowed by `kind` to access variant members.

```ds
struct User { name: string }

const descriptor = User;
if (descriptor.kind == "struct") {
    descriptor.properties[0].name satisfies string;
}
```

### descriptor descriptions are optional

> Doc comments map to optional descriptor descriptions.

```ds
/// User docs.
struct User {
    /// Name docs.
    name: string,
}

const descriptor = User;
if (descriptor.kind == "struct") {
    descriptor.description satisfies string | undefined;
    descriptor.properties[0].description satisfies string | undefined;
}
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
