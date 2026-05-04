# Struct Annotations

Struct annotations are resolved expressions on struct and field declarations.

## metadata

### struct metadata preserves the struct type

> Metadata on a struct does not change construction or field access.

```ds
newtype Label = { value: string };

function label(value: string): Label {
    Label({ value })
}

@label("user")
struct User {
    name: string;
}

const user = User { name: "Ada" };
user.name satisfies string;
```

### field metadata preserves the field type

> Metadata on a struct field does not change the field type.

```ds
newtype Label = { value: string };

function label(value: string): Label {
    Label({ value })
}

struct User {
    @label("display")
    name: string;
}

const user = User { name: "Ada" };
user.name satisfies string;
```

### field metadata can stack

> A field can carry multiple non-conflicting metadata annotations.

```ds
newtype Label = { value: string };

function label(value: string): Label {
    Label({ value })
}

struct User {
    @label("display")
    @cold
    name: string;
}

const user = User { name: "Ada" };
user.name satisfies string;
```

## resolution

### unresolved struct annotations are rejected

> The expression after `@` must resolve.

```ds
@missing
struct User {
    name: string;
}
```

- contains: missing

## rejections

### conflicting struct hints are rejected

> One struct declaration cannot carry contradictory branch hints.

```ds
@likely
@unlikely
struct User {
    name: string;
}
```

- contains: likely and unlikely

### conflicting field hints are rejected

> One field cannot carry contradictory branch hints.

```ds
struct User {
    @likely
    @unlikely
    name: string;
}
```

- contains: likely and unlikely
