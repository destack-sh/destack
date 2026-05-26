# Object Patterns

## object patterns

### object patterns bind fields

Object patterns bind structural fields by name.

```ds
let { x, y } = { x: 1, y: 2 };
x satisfies number;
y satisfies number;
```

### nominal object patterns bind struct fields

Nominal object patterns use the nominal tag.

```ds
struct Point {
    x: int32;
    y: int32;
}

let Point { x, y } = Point { x: 1, y: 2 };
x satisfies int32;
y satisfies int32;
```

### nominal object patterns bind class fields

Class patterns use the class tag and bind stored fields.

```ds
class User {
    name: string = "";
}

declare const user: User;

let User { name } = user;
name satisfies string;
```

### nominal object patterns reject getters

Object patterns bind stored fields, not getters.

```ds
class User {
    name: string = "";

    get displayName(): string {
        return this.name;
    }
}

declare const user: User;

let User { displayName } = user;
```

- contains: stored field

### bare object patterns reject nominal values

Untagged object patterns do not destructure nominal values.

```ds
struct Point {
    x: int32;
    y: int32;
}

let point = Point { x: 1, y: 2 };
let { x, y } = point;
```

- contains: not assignable

### object destructuring requires an initializer

Destructuring declarations require an initializer.

```ds
const { x }: { x: number };
```

- contains: destructuring declarations require initializers

## defaults

### object defaults fill absent fields

Defaults bind when the matched field is absent.

```ds
let { name = "Ada" } = {};
name satisfies string;
```

### object defaults bind aliases

Defaults can be attached to aliased fields.

```ds
let { name: displayName = "Ada" } = {};
displayName satisfies string;
```

## rest

### object rest binds tails

Rest patterns collect fields not named earlier in the pattern.

```ds
let { id, ...rest } = { id: 1, name: "Ada", active: true };
id satisfies number;
rest satisfies { name: string; active: boolean };
```

### object rest is last

Rest patterns cannot be followed by more fields.

```ds
let { ...rest, id } = { id: 1, name: "Ada" };
```

- contains: rest
