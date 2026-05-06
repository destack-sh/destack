# Type

## values

### type expressions coerce to reflected types

A type expression can be used where `Type<T>` is expected.

```ds
struct User {
    name: string;
}

const type: Type<User> = User;
type satisfies Type<User>;
```

### Type.of creates reflected types

`Type.of<T>()` returns the reflected type for `T`.

```ds
struct User {
    name: string;
}

const type = Type.of<User>();
type satisfies Type<User>;
type.id satisfies TypeId;
```

### reflected types are static values

`Type<T>` values can be passed as static generic values.

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

## layout

### layout queries return target-sized values

Layout queries are target-sensitive reflection operations.

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

### layoutOf returns field layouts

`layoutOf<T>()` returns field layout information for the active target.

```ds
struct Header {
    id: uint32;
}

const layout = comptime layoutOf<Header>();
layout satisfies Layout;
layout.fields satisfies readonly LayoutField[];
```
