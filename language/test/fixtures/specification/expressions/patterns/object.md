# Object Patterns

## object patterns

### object patterns bind fields

> Object patterns bind properties by name.

```ds
let { x, y } = { x: 1, y: 2 };
x satisfies int32;
y satisfies int32;
```

### tagged object patterns destructure structs

> Struct destructuring requires the struct tag.

```ds
struct Point {
    x: int32
    y: int32
}

let Point { x, y } = Point { x: 1, y: 2 };
x satisfies int32;
y satisfies int32;
```

### object patterns reject struct values

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

```ts
const { x }: { x: number };
```

- destructuring declarations require initializers

### object patterns bind readonly named identifiers

> `readonly` remains an identifier in object destructuring patterns.

```ts
const { readonly } = { readonly: 1 };
readonly satisfies number;
```

### object patterns reject readonly modifier syntax

> Pattern bindings do not support `readonly` modifier syntax.

```ds
let { readonly value } = { readonly: 1 };
```

- parse error: unexpected Identifier in Expression

## defaults

### object defaults fill missing fields

> Default values are used when the matched field is absent.

```ds
let { name = "Ada" } = {};
name satisfies string;
```

### object defaults keep aliases

> Defaults can be attached to aliased fields.

```ds
let { name: displayName = "Ada" } = {};
displayName satisfies string;
```

## rest

### object rest binds remaining fields

> Rest patterns collect fields not named earlier in the pattern.

```ds
let { id, ...rest } = { id: 1, name: "Ada", active: true };
id satisfies int32;
rest satisfies { name: string, active: boolean };
```

### object rest must be last

> Rest patterns cannot be followed by more fields.

```ds
let { ...rest, id } = { id: 1, name: "Ada" };
```

- contains: rest
