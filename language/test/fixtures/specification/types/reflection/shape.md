# Shape

## types

### Type.of returns a semantic type

`Type.of<T>()` returns the normalized DIR semantic type for `T`.

```ds
struct User {
    name: string;
}

const type = Type.of<User>();
(type) satisfies Type<User>;
(type) satisfies dir.Type;
type.id satisfies TypeId;
```

### reference types expose symbols

Nominal types reflect as references to DIR symbols.

```ds
struct User {
    name: string;
}

const type = Type.of<User>();

if (type.kind == "reference") {
    (type) satisfies dir.ReferenceType;
    type.symbol satisfies dir.Symbol;
}
```

### object types expose fields

Structural object types reflect through DIR object fields.

```ds
const type = Type.of<type { name: string; age?: uint }>();

if (type.kind == "object") {
    (type) satisfies dir.ObjectType;
    type.fields satisfies readonly dir.TypeField[];
}
```

### fixed arrays expose element and count

Fixed arrays reflect as sized array types.

```ds
const type = Type.of<[uint8; 4]>();

if (type.kind == "arraySized") {
    (type) satisfies dir.ArraySizedType;
    type.element satisfies dir.Type;
    type.count satisfies dir.Type;
}
```

### tuples expose ordered elements

Tuple types reflect through DIR tuple elements.

```ds
const type = Type.of<(string, int32)>();

if (type.kind == "tuple") {
    (type) satisfies dir.TupleType;
    type.elements satisfies readonly dir.TypeElement[];
}
```

### unions expose members

Union types reflect through DIR union elements.

```ds
const type = Type.of<string | int32>();

if (type.kind == "union") {
    (type) satisfies dir.UnionType;
    type.elements satisfies readonly dir.Type[];
}
```
