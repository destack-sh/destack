# Type Reflection

## handles

### type expressions can coerce to type handles

> A type expression can be used where `Type<T>` is expected.

```ds
struct User {
    name: string;
}

const handle: Type<User> = User;
handle satisfies Type<User>;
```

### type of creates type handles

> `Type.of<T>()` returns a handle for `T`.

```ds
struct User {
    name: string;
}

const handle = Type.of<User>();
handle satisfies Type<User>;
```

### reflected types can be generic comptime arguments

> `Type<T>` handles can be passed as comptime generic parameters.

```ds
declare function parseWithType<comptime T: Type>(raw: string): T;

function parse<comptime T: Type>(raw: string): T {
    parseWithType<T>(raw)
}

struct User {
    name: string;
}

const user = parse<User>("{}");
user satisfies User;
```

## shape

### shape of returns a type shape

> `shapeOf` exposes the stable reflection shape.

```ds
struct User {
    name: string;
}

const shape = shapeOf(Type.of<User>());
shape satisfies TypeShape;
```

### display name returns a string

> `displayNameOf` returns a human-readable type name.

```ds
struct User {
    name: string;
}

const name = displayNameOf(Type.of<User>());
name satisfies string;
```

## layout

### layout intrinsics return target sized values

> Layout queries are target-sensitive reflection operations.

```ds
struct Header {
    id: uint32;
}

const size = comptime sizeOf<Header>();
const alignment = comptime alignOf<Header>();
const stride = comptime strideOf<Header>();

size satisfies usize;
alignment satisfies usize;
stride satisfies usize;
```

### layout of returns a layout descriptor

> `layoutOf<T>()` returns field layout information for the active target.

```ds
struct Header {
    id: uint32;
}

const layout = comptime layoutOf<Header>();
layout satisfies LayoutDescriptor;
```
