# Object Patterns

## object patterns

### object patterns bind fields

> Object patterns bind properties by name.

```ds
let { x, y } = { x: 1, y: 2 };
x satisfies int32;
y satisfies int32;
```

### struct object patterns need tags

> Struct destructuring uses the struct tag.

```ds
struct Point {
    x: int32
    y: int32
}

let Point { x, y } = Point { x: 1, y: 2 };
x satisfies int32;
y satisfies int32;
```

### bare object patterns reject structs

> Untagged object patterns do not destructure nominal structs.

```ds
struct Point {
    x: int32
    y: int32
}

let point = Point { x: 1, y: 2 };
let { x, y } = point;
```

- contains: not assignable

### object destructuring requires an initializer

> Destructuring declarations require an initializer.

```ds
const { x }: { x: number };
```

- contains: destructuring declarations require initializers

### object patterns reject readonly modifiers

> `readonly` is not valid in pattern bindings.

```ds
let { readonly value } = { readonly: 1 };
```

- contains: parse error: unexpected Identifier in Expression

## defaults

### object defaults fill absent fields

> Defaults bind when the matched field is absent.

```ds
let { name = "Ada" } = {};
name satisfies string;
```

### object defaults bind aliases

> Defaults can be attached to aliased fields.

```ds
let { name: displayName = "Ada" } = {};
displayName satisfies string;
```

## rest

### object rest binds tails

> Rest patterns collect fields not named earlier in the pattern.

```ds
let { id, ...rest } = { id: 1, name: "Ada", active: true };
id satisfies int32;
rest satisfies { name: string, active: boolean };
```

### object rest is last

> Rest patterns cannot be followed by more fields.

```ds
let { ...rest, id } = { id: 1, name: "Ada" };
```

- contains: rest
