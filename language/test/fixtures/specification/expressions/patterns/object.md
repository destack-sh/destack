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

- contains: destructuring declarations require initializers
